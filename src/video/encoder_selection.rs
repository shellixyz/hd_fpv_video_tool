//! Chooses the video encoder: the codec and whether to encode it in hardware.

use super::{Codec, HwAcceleratedEncoding, resolution::Resolution};

/// A device's hardware encoder for one codec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HardwareEncoder {
	/// The largest frame the encoder accepts, or `None` when the device does not report one.
	pub max_frame_size: Option<Resolution>,
}

/// What the hardware encoders of a device can do.
pub trait EncoderCaps {
	/// The device's hardware encoder for `codec`, or `None` when it has none.
	fn hardware_encoder(&self, codec: Codec) -> Option<HardwareEncoder>;
}

/// The chosen encoder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncoderChoice {
	pub codec: Codec,
	pub hw_acceleration: HwAcceleratedEncoding,
	/// Hardware encoders passed over because the frame exceeds their maximum size.
	pub oversized: Vec<OversizedFrame>,
}

/// A hardware encoder passed over because the frame exceeds its maximum size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, derive_more::Display)]
#[display("{frame_size} frames exceed the {codec} hardware encoder's maximum of {max_frame_size}")]
pub struct OversizedFrame {
	pub codec: Codec,
	pub frame_size: Resolution,
	pub max_frame_size: Resolution,
}

/// Hardware codecs tried, in order, when the user does not request one.
const HARDWARE_DEFAULT_PREFERENCE: [Codec; 2] = [Codec::AV1, Codec::H265];

/// The encoder used when no hardware encoder is chosen and the user does not request a codec.
const SOFTWARE_DEFAULT_CODEC: Codec = Codec::H265;

/// Chooses the codec and hardware mode for encoding frames of `frame_size`.
///
/// `requested` is the codec the user asked for, if any. `caps` is `None` when hardware encoding
/// is disabled or unavailable.
#[must_use]
pub fn select_encoder(
	requested: Option<Codec>,
	frame_size: Resolution,
	caps: Option<&dyn EncoderCaps>,
) -> EncoderChoice {
	let mut oversized = Vec::new();
	let mut usable = |codec| {
		let Some(encoder) = caps.and_then(|caps| caps.hardware_encoder(codec)) else {
			return false;
		};
		match encoder.max_frame_size {
			Some(max_frame_size)
				if frame_size.width() > max_frame_size.width() || frame_size.height() > max_frame_size.height() =>
			{
				oversized.push(OversizedFrame {
					codec,
					frame_size,
					max_frame_size,
				});
				false
			},
			_ => true,
		}
	};
	let (codec, hw_acceleration) = match requested {
		Some(codec) => (codec, HwAcceleratedEncoding::from(usable(codec))),
		None => HARDWARE_DEFAULT_PREFERENCE
			.into_iter()
			.find(|&codec| usable(codec))
			.map_or((SOFTWARE_DEFAULT_CODEC, HwAcceleratedEncoding::No), |codec| {
				(codec, HwAcceleratedEncoding::Yes)
			}),
	};
	for frame in &oversized {
		if requested.is_some() {
			log::warn!("{frame}, encoding {codec} in software instead");
		} else {
			log::info!("{frame}, skipping it");
		}
	}
	if requested.is_none() && !oversized.is_empty() {
		let mode = if hw_acceleration.is_yes() {
			"hardware"
		} else {
			"software"
		};
		log::info!("encoding {codec} in {mode}");
	}
	EncoderChoice {
		codec,
		hw_acceleration,
		oversized,
	}
}

/// The hardware encoders of this machine's VA-API device, or `None` when `no_hwaccel` is set,
/// the `hwaccel` feature is disabled, or the device cannot be opened.
#[must_use]
pub fn device_caps(no_hwaccel: bool) -> Option<Box<dyn EncoderCaps>> {
	#[cfg(feature = "hwaccel")]
	if !no_hwaccel {
		return super::hw_accel::vaapi_cap_finder().map(|caps| Box::new(caps) as Box<dyn EncoderCaps>);
	}
	let _ = no_hwaccel;
	None
}

#[cfg(test)]
pub(crate) mod test_support {
	use super::{Codec, EncoderCaps, HardwareEncoder, Resolution};

	/// A device with hardware encoders for the listed codecs, each with an optional maximum frame
	/// size.
	pub(crate) struct FakeCaps(pub(crate) Vec<(Codec, Option<Resolution>)>);

	impl FakeCaps {
		/// Hardware encoders for `codecs` that report no maximum frame size.
		pub(crate) fn unlimited(codecs: &[Codec]) -> Self {
			Self(codecs.iter().map(|&codec| (codec, None)).collect())
		}
	}

	impl EncoderCaps for FakeCaps {
		fn hardware_encoder(&self, codec: Codec) -> Option<HardwareEncoder> {
			self.0
				.iter()
				.find(|(candidate, _)| *candidate == codec)
				.map(|&(_, max_frame_size)| HardwareEncoder { max_frame_size })
		}
	}
}

#[cfg(test)]
mod tests {
	use super::{test_support::FakeCaps, *};

	const FRAME_1080P: Resolution = Resolution::new(1920, 1080);

	/// The codec and hardware mode of `choice`.
	fn chosen(choice: &EncoderChoice) -> (Codec, HwAcceleratedEncoding) {
		(choice.codec, choice.hw_acceleration)
	}

	#[test]
	fn the_default_prefers_hardware_av1() {
		let caps = FakeCaps::unlimited(&[Codec::H265, Codec::AV1]);
		assert_eq!(
			chosen(&select_encoder(None, FRAME_1080P, Some(&caps))),
			(Codec::AV1, HwAcceleratedEncoding::Yes)
		);
	}

	#[test]
	fn the_default_skips_a_hardware_encoder_narrower_than_the_frame() {
		let caps = FakeCaps(vec![
			(Codec::AV1, Some(Resolution::new(4096, 4096))),
			(Codec::H265, Some(Resolution::new(8192, 4096))),
		]);
		assert_eq!(
			chosen(&select_encoder(None, Resolution::new(4212, 1188), Some(&caps))),
			(Codec::H265, HwAcceleratedEncoding::Yes)
		);
	}

	#[test]
	fn the_default_skips_a_hardware_encoder_shorter_than_the_frame() {
		let caps = FakeCaps(vec![
			(Codec::AV1, Some(Resolution::new(4096, 2160))),
			(Codec::H265, Some(Resolution::new(4096, 4096))),
		]);
		assert_eq!(
			chosen(&select_encoder(None, Resolution::new(2160, 3840), Some(&caps))),
			(Codec::H265, HwAcceleratedEncoding::Yes)
		);
	}

	#[test]
	fn an_explicit_codec_too_large_for_its_hardware_encoder_is_encoded_in_software_and_says_why() {
		let max_frame_size = Resolution::new(4096, 2304);
		let caps = FakeCaps(vec![(Codec::AV1, Some(max_frame_size))]);
		let frame_size = Resolution::new(4212, 1188);
		assert_eq!(
			select_encoder(Some(Codec::AV1), frame_size, Some(&caps)),
			EncoderChoice {
				codec: Codec::AV1,
				hw_acceleration: HwAcceleratedEncoding::No,
				oversized: vec![OversizedFrame {
					codec: Codec::AV1,
					frame_size,
					max_frame_size,
				}],
			}
		);
	}

	#[test]
	fn the_default_is_software_h265_when_the_frame_is_too_large_for_every_hardware_encoder() {
		let max_frame_size = Resolution::new(4096, 2304);
		let caps = FakeCaps(vec![
			(Codec::AV1, Some(max_frame_size)),
			(Codec::H265, Some(max_frame_size)),
		]);
		let frame_size = Resolution::new(4212, 1188);
		let oversized = |codec| OversizedFrame {
			codec,
			frame_size,
			max_frame_size,
		};
		assert_eq!(
			select_encoder(None, frame_size, Some(&caps)),
			EncoderChoice {
				codec: Codec::H265,
				hw_acceleration: HwAcceleratedEncoding::No,
				oversized: vec![oversized(Codec::AV1), oversized(Codec::H265)],
			}
		);
	}

	#[test]
	fn the_default_falls_back_to_hardware_h265_without_hardware_av1() {
		let caps = FakeCaps::unlimited(&[Codec::H264, Codec::H265]);
		assert_eq!(
			chosen(&select_encoder(None, FRAME_1080P, Some(&caps))),
			(Codec::H265, HwAcceleratedEncoding::Yes)
		);
	}

	#[test]
	fn the_default_is_software_h265_when_the_hardware_has_neither_av1_nor_h265() {
		let caps = FakeCaps::unlimited(&[Codec::H264]);
		assert_eq!(
			chosen(&select_encoder(None, FRAME_1080P, Some(&caps))),
			(Codec::H265, HwAcceleratedEncoding::No)
		);
	}

	#[test]
	fn an_explicit_codec_without_a_hardware_encoder_is_encoded_in_software() {
		let caps = FakeCaps::unlimited(&[Codec::AV1]);
		assert_eq!(
			chosen(&select_encoder(Some(Codec::VP8), FRAME_1080P, Some(&caps))),
			(Codec::VP8, HwAcceleratedEncoding::No)
		);
	}

	#[test]
	fn an_explicit_codec_uses_its_hardware_encoder_when_the_frame_fits_its_limit() {
		let caps = FakeCaps(vec![(Codec::H264, Some(Resolution::new(4096, 4096)))]);
		assert_eq!(
			chosen(&select_encoder(
				Some(Codec::H264),
				Resolution::new(4096, 2160),
				Some(&caps)
			)),
			(Codec::H264, HwAcceleratedEncoding::Yes)
		);
	}

	#[test]
	fn an_explicit_codec_shorter_than_the_frame_is_encoded_in_software() {
		let caps = FakeCaps(vec![(Codec::H265, Some(Resolution::new(4096, 2160)))]);
		assert_eq!(
			chosen(&select_encoder(
				Some(Codec::H265),
				Resolution::new(2160, 3840),
				Some(&caps)
			)),
			(Codec::H265, HwAcceleratedEncoding::No)
		);
	}

	#[test]
	fn the_fallback_reason_names_the_codec_the_frame_size_and_the_limit() {
		let frame = OversizedFrame {
			codec: Codec::AV1,
			frame_size: Resolution::new(4212, 1188),
			max_frame_size: Resolution::new(4096, 2304),
		};
		assert_eq!(
			frame.to_string(),
			"4212x1188 frames exceed the AV1 hardware encoder's maximum of 4096x2304"
		);
	}

	#[test]
	fn an_explicit_codec_uses_its_hardware_encoder() {
		let caps = FakeCaps::unlimited(&[Codec::AV1, Codec::H264]);
		assert_eq!(
			chosen(&select_encoder(Some(Codec::H264), FRAME_1080P, Some(&caps))),
			(Codec::H264, HwAcceleratedEncoding::Yes)
		);
	}

	#[test]
	fn without_hardware_the_default_is_software_h265() {
		assert_eq!(
			chosen(&select_encoder(None, FRAME_1080P, None)),
			(Codec::H265, HwAcceleratedEncoding::No)
		);
	}

	#[test]
	fn without_hardware_an_explicit_codec_is_encoded_in_software() {
		assert_eq!(
			chosen(&select_encoder(Some(Codec::VP9), FRAME_1080P, None)),
			(Codec::VP9, HwAcceleratedEncoding::No)
		);
	}
}
