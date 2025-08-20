use derive_more::derive::IsVariant;

use crate::AsBool;

#[cfg(feature = "hwaccel")]
pub mod vaapi_cap_finder;

#[cfg(feature = "hwaccel")]
pub use vaapi_cap_finder::{VaapiCapFinder, vaapi_cap_finder};

#[derive(Debug, Clone, Copy, PartialEq, Eq, derive_more::Display, IsVariant)]
pub enum HwAcceleratedEncoding {
	Yes,
	No,
}

impl From<bool> for HwAcceleratedEncoding {
	fn from(b: bool) -> Self {
		if b { Self::Yes } else { Self::No }
	}
}

impl AsBool for HwAcceleratedEncoding {
	fn as_bool(&self) -> bool {
		*self == Self::Yes
	}
}
