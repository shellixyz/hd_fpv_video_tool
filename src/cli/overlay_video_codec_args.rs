use crate::{
	osd::overlay::OverlayVideoCodec,
	video::{self, HwAcceleratedEncoding},
};
use clap::Args;
use getset::CopyGetters;

#[derive(Args, Clone, CopyGetters)]
pub struct OverlayVideoCodecArgs {
	#[clap(short, long, default_value_t = false)]
	#[getset(get_copy = "pub")]
	no_hwaccel: bool,

	#[clap(short, long)]
	codec: Option<OverlayVideoCodec>,
}

impl OverlayVideoCodecArgs {
	pub fn codec(&self) -> (OverlayVideoCodec, HwAcceleratedEncoding) {
		const FALLBACK: (OverlayVideoCodec, HwAcceleratedEncoding) =
			(OverlayVideoCodec::VP8, HwAcceleratedEncoding::No);
		match self.codec {
			Some(_) | None if self.no_hwaccel => FALLBACK,
			Some(codec) => match video::hw_accel::vaapi_cap_finder() {
				Some(hw_accel_cap) => (
					codec,
					HwAcceleratedEncoding::from(hw_accel_cap.can_encode(video::Codec::from(codec))),
				),
				None => (codec, HwAcceleratedEncoding::No),
			},
			None => {
				let hw_accel_codec = video::hw_accel::vaapi_cap_finder().and_then(|hw_accel_cap| {
					[OverlayVideoCodec::HEVC, OverlayVideoCodec::VP9, OverlayVideoCodec::VP8]
						.iter()
						.find(|&codec| hw_accel_cap.can_encode(video::Codec::from(*codec)))
				});
				if let Some(hw_accel_codec) = hw_accel_codec {
					(*hw_accel_codec, HwAcceleratedEncoding::Yes)
				} else {
					FALLBACK
				}
			},
		}
	}
}
