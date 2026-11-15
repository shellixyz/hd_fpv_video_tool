use std::{env, path::PathBuf, rc::Rc};

use cros_libva::{VA_ATTRIB_NOT_SUPPORTED, VAConfigAttrib, VAConfigAttribType, VAEntrypoint, VAProfile};

use crate::video::{
	Codec,
	encoder_selection::{EncoderCaps, HardwareEncoder},
	resolution::Resolution,
};

pub struct VaapiCapFinder {
	display: Rc<cros_libva::Display>,
	drm_device_path: PathBuf,
}

impl VaapiCapFinder {
	#[must_use]
	pub fn new() -> Option<Self> {
		unsafe { env::set_var("LIBVA_MESSAGING_LEVEL", "0") };
		// Probe render nodes ourselves so we can remember which device was opened.
		for idx in 128..192u32 {
			let path = PathBuf::from(format!("/dev/dri/renderD{idx}"));
			if !path.exists() {
				break;
			}
			if let Ok(display) = cros_libva::Display::open_drm_display(&path) {
				return Some(Self {
					display,
					drm_device_path: path,
				});
			}
		}
		None
	}

	/// Returns the DRM device node that was used to open the VA-API display,
	/// e.g. `/dev/dri/renderD128`. Pass this to ffmpeg's `-vaapi_device` argument.
	#[must_use]
	pub fn drm_device_path(&self) -> &PathBuf {
		&self.drm_device_path
	}

	/// The largest frame the encoder for `profile` and `entrypoint` accepts, or `None` when the
	/// driver reports neither a maximum width nor a maximum height. A dimension the driver does not
	/// report is unlimited.
	fn max_frame_size(&self, profile: VAProfile::Type, entrypoint: VAEntrypoint::Type) -> Option<Resolution> {
		let mut attributes = [
			VAConfigAttribType::VAConfigAttribMaxPictureWidth,
			VAConfigAttribType::VAConfigAttribMaxPictureHeight,
		]
		.map(|type_| VAConfigAttrib { type_, value: 0 });
		self.display
			.get_config_attributes(profile, entrypoint, &mut attributes)
			.ok()?;
		let [width, height] =
			attributes.map(|attribute| (attribute.value != VA_ATTRIB_NOT_SUPPORTED).then_some(attribute.value));
		if width.is_none() && height.is_none() {
			return None;
		}
		Some(Resolution::new(width.unwrap_or(u32::MAX), height.unwrap_or(u32::MAX)))
	}
}

impl EncoderCaps for VaapiCapFinder {
	fn hardware_encoder(&self, codec: Codec) -> Option<HardwareEncoder> {
		let va_profile = match codec {
			Codec::AV1 => VAProfile::VAProfileAV1Profile0,
			Codec::H264 => VAProfile::VAProfileH264High,
			Codec::H265 => VAProfile::VAProfileHEVCMain,
			Codec::VP8 => VAProfile::VAProfileVP8Version0_3,
			Codec::VP9 => VAProfile::VAProfileVP9Profile0,
		};
		let entrypoints = self.display.query_config_entrypoints(va_profile).ok()?;
		// FFmpeg prefers the full-power entrypoint and falls back to the low-power one.
		let entrypoint = [VAEntrypoint::VAEntrypointEncSlice, VAEntrypoint::VAEntrypointEncSliceLP]
			.into_iter()
			.find(|entrypoint| entrypoints.contains(entrypoint))?;
		Some(HardwareEncoder {
			max_frame_size: self.max_frame_size(va_profile, entrypoint),
		})
	}
}

#[must_use]
pub fn vaapi_cap_finder() -> Option<VaapiCapFinder> {
	let res = VaapiCapFinder::new();
	if res.is_none() {
		log::warn!("could not access VA-API through libva, hardware acceleration disabled");
	}
	res
}

/// Returns the path of the first usable DRM render node, e.g. `/dev/dri/renderD128`.
///
/// This is used to pass `-vaapi_device <path>` to ffmpeg so it can upload frames
/// to the correct VA-API device. Returns `None` if no render node is found.
#[must_use]
pub fn vaapi_device_path() -> Option<PathBuf> {
	for idx in 128..192u32 {
		let path = PathBuf::from(format!("/dev/dri/renderD{idx}"));
		if !path.exists() {
			break;
		}
		if cros_libva::Display::open_drm_display(&path).is_ok() {
			return Some(path);
		}
	}
	None
}
