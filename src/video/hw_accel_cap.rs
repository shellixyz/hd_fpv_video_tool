use std::{borrow::Borrow, env, rc::Rc};

use cros_libva::{VAEntrypoint, VAProfile};

use super::Codec;

pub struct HwAccelCap(Rc<cros_libva::Display>);

impl HwAccelCap {
	pub fn new() -> Option<Self> {
		env::set_var("LIBVA_MESSAGING_LEVEL", "0");
		let display = cros_libva::Display::open()?;
		Some(Self(display))
	}

	pub fn can_encode(&self, codec: impl Borrow<Codec>) -> bool {
		let va_profile = match codec.borrow() {
			Codec::AV1 => VAProfile::VAProfileAV1Profile0,
			Codec::H264 => VAProfile::VAProfileH264High,
			Codec::H265 => VAProfile::VAProfileHEVCMain,
			Codec::VP8 => VAProfile::VAProfileVP8Version0_3,
			Codec::VP9 => VAProfile::VAProfileVP9Profile0,
		};
		match self.0.query_config_entrypoints(va_profile) {
			Ok(entrypoints) => [VAEntrypoint::VAEntrypointEncSlice, VAEntrypoint::VAEntrypointEncSliceLP]
				.iter()
				.any(|&entrypoint| entrypoints.contains(&entrypoint)),
			Err(_) => false,
		}
	}
}
