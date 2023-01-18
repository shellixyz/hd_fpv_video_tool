
use strum::{Display, EnumIter};

#[derive(Debug, Display, Clone, Copy, EnumIter, PartialEq, Eq, Hash)]
pub enum FontVariant {
    Generic,
    Ardupilot,
    Betaflight,
    INAV,
    KISSUltra,
    Unknown
}

impl FontVariant {
    pub fn font_set_ident(&self) -> Option<&str> {
        use FontVariant::*;
        match self {
            Ardupilot => Some("ardu"),
            INAV => Some("inav"),
            Betaflight => Some("bf"),
            KISSUltra => Some("ultra"),
            Generic | Unknown => None,
        }
    }
}

