use std::{borrow::Borrow, env, path::PathBuf, rc::Rc};

use cros_libva::{VAEntrypoint, VAProfile};

use crate::video::Codec;

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
	pub fn drm_device_path(&self) -> &PathBuf {
		&self.drm_device_path
	}

	pub fn can_encode(&self, codec: impl Borrow<Codec>) -> bool {
		let va_profile = match codec.borrow() {
			Codec::AV1 => VAProfile::VAProfileAV1Profile0,
			Codec::H264 => VAProfile::VAProfileH264High,
			Codec::H265 => VAProfile::VAProfileHEVCMain,
			Codec::VP8 => VAProfile::VAProfileVP8Version0_3,
			Codec::VP9 => VAProfile::VAProfileVP9Profile0,
		};
		match self.display.query_config_entrypoints(va_profile) {
			Ok(entrypoints) => [VAEntrypoint::VAEntrypointEncSlice, VAEntrypoint::VAEntrypointEncSliceLP]
				.iter()
				.any(|&entrypoint| entrypoints.contains(&entrypoint)),
			Err(_) => false,
		}
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
