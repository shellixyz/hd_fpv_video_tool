

use hd_fpv_osd_font_tool::prelude::tile;
use thiserror::Error;

use super::{dji, wsa, Dimensions};


#[derive(Debug, strum::Display, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum Kind {
    DJI_SD,
    DJI_FakeHD,
    DJI_HD,
    WSA,
}

impl Kind {

    pub const fn dimensions_tiles(&self) -> Dimensions {
        use Kind::*;
        match self {
            DJI_SD => dji::dimensions::SD,
            DJI_FakeHD => dji::dimensions::FAKE_HD,
            DJI_HD => dji::dimensions::HD,
            WSA => wsa::DIMENSIONS,
        }
    }

    pub const fn tile_kind(&self) -> tile::Kind {
        use Kind::*;
        match self {
            DJI_SD => tile::Kind::SD,
            DJI_FakeHD => tile::Kind::HD,
            DJI_HD => tile::Kind::HD,
            WSA => tile::Kind::SD,
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
            _ => Err(InvalidDimensionsError(*dimensions_tiles))
        }
    }
}
