
use std::{error::Error, fmt::Display};

use hd_fpv_osd_font_tool::osd::tile;

pub mod file;

pub type Dimensions = hd_fpv_osd_font_tool::dimensions::Dimensions<u32>;

pub mod dimensions {
    use super::Dimensions;
    pub const SD: Dimensions = Dimensions::new(30, 15);
    pub const FAKE_HD: Dimensions = Dimensions::new(60, 22);
    pub const HD: Dimensions = Dimensions::new(50, 18);
}

#[derive(Debug, strum::Display, Clone, Copy)]
pub enum Kind {
    SD,
    FakeHD,
    HD
}

impl Kind {

    pub const fn dimensions_tiles(&self) -> Dimensions {
        use Kind::*;
        match self {
            SD => dimensions::SD,
            FakeHD => dimensions::FAKE_HD,
            HD => dimensions::HD,
        }
    }

    pub const fn tile_kind(&self) -> tile::Kind {
        use Kind::*;
        match self {
            SD => tile::Kind::SD,
            FakeHD => tile::Kind::HD,
            HD => tile::Kind::HD,
        }
    }

    pub const fn dimensions_pixels(&self) -> (u32, u32) {
        let dimensions_tiles = self.dimensions_tiles();
        let tile_dimensions = self.tile_kind().dimensions();
        (dimensions_tiles.width * tile_dimensions.width, dimensions_tiles.height * tile_dimensions.height)
    }

}

#[derive(Debug)]
pub struct InvalidDimensionsError(pub Dimensions);
impl Error for InvalidDimensionsError {}

impl Display for InvalidDimensionsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid dimensions tiles: {}x{}", self.0.width(), self.0.height())
    }
}


impl TryFrom<&Dimensions> for Kind {
    type Error = InvalidDimensionsError;

    fn try_from(dimensions_tiles: &Dimensions) -> Result<Self, Self::Error> {
        match *dimensions_tiles {
            dimensions::SD => Ok(Self::SD),
            dimensions::FAKE_HD => Ok(Self::FakeHD),
            dimensions::HD => Ok(Self::HD),
            _ => Err(InvalidDimensionsError(*dimensions_tiles))
        }
    }
}
