use strum::{Display, EnumIter};

#[derive(Debug, Display, Clone, Copy, EnumIter, PartialEq, Eq, Hash)]
pub enum FontVariant {
	Generic,
	Ardupilot,
	Betaflight,
	INAV,
	KISSUltra,
	Unknown,
}

impl FontVariant {
	#[must_use]
	pub fn font_set_ident(&self) -> Option<&str> {
		use FontVariant as FV;
		match self {
			FV::Ardupilot => Some("ardu"),
			FV::INAV => Some("inav"),
			FV::Betaflight => Some("bf"),
			FV::KISSUltra => Some("ultra"),
			FV::Generic | FV::Unknown => None,
		}
	}
}
