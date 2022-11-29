
use thiserror::Error;

use hd_fpv_osd_font_tool::prelude::*;

use crate::video::resolution::Resolution as VideoResolution;

pub mod file;
pub mod font_dir;


pub const AU_OSD_FRAME_SHIFT: i32 = -36;

#[derive(Debug, Error)]
#[error("video resolution {video_resolution} too small to fit {osd_kind} kind OSD")]
pub struct VideoResolutionTooSmallError {
    pub osd_kind: Kind,
    pub video_resolution: VideoResolution
}

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

}

#[derive(Debug, Error)]
#[error("invalid dimensions tiles: {0}")]
pub struct InvalidDimensionsError(pub Dimensions);

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
