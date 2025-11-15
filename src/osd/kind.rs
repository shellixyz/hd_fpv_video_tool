use hd_fpv_osd_font_tool::prelude::tile;
use thiserror::Error;

use super::{Dimensions, dji, wsa};

#[derive(Debug, strum::Display, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum Kind {
	DJI_SD,
	DJI_FakeHD,
	DJI_HD,
	WSA,
}

impl Kind {
	#[must_use]
	pub const fn dimensions_tiles(&self) -> Dimensions {
		match self {
			Kind::DJI_SD => dji::dimensions::SD,
			Kind::DJI_FakeHD => dji::dimensions::FAKE_HD,
			Kind::DJI_HD => dji::dimensions::HD,
			Kind::WSA => wsa::DIMENSIONS,
		}
	}

	#[must_use]
	pub const fn tile_kind(&self) -> tile::Kind {
		match self {
			Kind::DJI_FakeHD | Kind::DJI_HD => tile::Kind::HD,
			Kind::DJI_SD | Kind::WSA => tile::Kind::SD,
		}
	}
}

#[derive(Debug, Error)]
#[error("invalid dimensions tiles: {0}")]
pub struct InvalidDimensionsError(pub Dimensions);

impl TryFrom<&Dimensions> for Kind {
	type Error = InvalidDimensionsError;

	fn try_from(dimensions_tiles: &Dimensions) -> Result<Self, Self::Error> {
		match *dimensions_tiles {
			dji::dimensions::SD => Ok(Self::DJI_SD),
			dji::dimensions::FAKE_HD => Ok(Self::DJI_FakeHD),
			dji::dimensions::HD => Ok(Self::DJI_HD),
			_ => Err(InvalidDimensionsError(*dimensions_tiles)),
		}
	}
}
