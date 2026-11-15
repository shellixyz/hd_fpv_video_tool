use strum::{Display, EnumIter};

#[derive(Debug, Display, Clone, Copy, EnumIter, PartialEq, Eq, Hash)]
pub enum FontVariant {
	Generic,
	Ardupilot,
	Betaflight,
	INAV,
	KISSUltra,
	Quicksilver,
	Unknown,
}

impl FontVariant {
	/// Maps an MSP FC variant ID, as recorded by Walksnail and by msp-osd's DJI format v2, to its
	/// font variant. `INAV` and `ARDU` are seen in Walksnail recordings; the other IDs are the MSP
	/// FC variants msp-osd recognises.
	#[must_use]
	pub fn from_msp_fc_variant_id(id: &str) -> Self {
		use FontVariant as FV;
		match id {
			"BTFL" => FV::Betaflight,
			"INAV" => FV::INAV,
			"ARDU" => FV::Ardupilot,
			"ULTR" => FV::KISSUltra,
			"QUIC" => FV::Quicksilver,
			_ => FV::Unknown,
		}
	}

	#[must_use]
	pub fn font_set_ident(&self) -> Option<&str> {
		use FontVariant as FV;
		match self {
			FV::Ardupilot => Some("ardu"),
			FV::INAV => Some("inav"),
			FV::Betaflight => Some("bf"),
			FV::KISSUltra => Some("ultra"),
			FV::Quicksilver | FV::Generic | FV::Unknown => None,
		}
	}

	/// The idents msp-osd looks up PNG fonts by, then the legacy aliases it accepts.
	#[must_use]
	pub fn png_font_idents(&self) -> &'static [&'static str] {
		use FontVariant as FV;
		match self {
			FV::Ardupilot => &["ardu"],
			FV::INAV => &["inav"],
			FV::Betaflight => &["btfl", "bf"],
			FV::KISSUltra => &["ultr", "ultra"],
			FV::Quicksilver => &["quic"],
			FV::Generic | FV::Unknown => &[],
		}
	}
}
