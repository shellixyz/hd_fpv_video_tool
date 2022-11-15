
use hd_fpv_osd_font_tool::osd::tile;
use thiserror::Error;

use super::frame_overlay::{VideoResolution, Resolution as FrameOverlayResolution};
use hd_fpv_osd_font_tool::dimensions::Dimensions as GenericDimensions;

use strum::IntoEnumIterator;

pub mod file;


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

mod utils {
    use super::GenericDimensions;

    pub(crate) fn dimensions_diff(d1: GenericDimensions<u32>, d2: GenericDimensions<u32>) -> (i32, i32) {
        (d1.width as i32 - d2.width as i32, d1.height as i32 - d2.height as i32)
    }

    pub(crate) fn margins(outside_dimensions: GenericDimensions<u32>, inside_dimensions: GenericDimensions<u32>) -> (i32, i32) {
        let (margin_width_x2, margin_height_x2) = dimensions_diff(outside_dimensions, inside_dimensions);
        (margin_width_x2 / 2, margin_height_x2 / 2)
    }
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

    pub fn dimensions_pixels_for_tile_kind(&self, tile_kind: tile::Kind) -> FrameOverlayResolution {
        self.dimensions_tiles() * tile_kind.dimensions()
    }

    pub fn dimensions_pixels_for_tile_dimensions(&self, tile_dimensions: tile::Dimensions) -> FrameOverlayResolution {
        self.dimensions_tiles() * tile_dimensions
    }

    /// Returns the best kind of tile to use without rescaling tiles so that the OSD fills as much as the screen as possible
    pub fn best_kind_of_tiles_to_use_without_scaling(&self, video_resolution: VideoResolution) -> Result<tile::Kind, VideoResolutionTooSmallError> {
        let avg_margins = tile::Kind::iter().flat_map(|tile_kind| {
            let osd_dimensions = self.dimensions_pixels_for_tile_kind(tile_kind);
            let (margin_width, margin_height) = utils::margins(video_resolution, osd_dimensions);
            if margin_width >= 0 && margin_height >= 0 {
                Some((tile_kind, (margin_width as u32 + margin_height as u32) / 2))
            } else {
                None
            }
        });
        match avg_margins.min_by_key(|(_, margin_avg)| *margin_avg) {
            Some((tile_kind, _)) => Ok(tile_kind),
            None => Err(VideoResolutionTooSmallError { osd_kind: *self, video_resolution })
        }
    }

    pub fn best_kind_of_tiles_to_use_with_scaling(&self, max_resolution: FrameOverlayResolution) -> (tile::Kind, tile::Dimensions, FrameOverlayResolution) {
        let max_tile_width = max_resolution.width / self.dimensions_tiles().width;
        let max_tile_height = max_resolution.height / self.dimensions_tiles().height;
        let tile_kinds_data = tile::Kind::iter().map(|tile_kind| {
            let width_diff = max_tile_width as i32 - tile_kind.dimensions().width as i32;
            let height_diff = max_tile_height as i32 - tile_kind.dimensions().height as i32;
            println!("{tile_kind}: wdiff {width_diff} - hdiff {height_diff}");
            (tile_kind, width_diff, height_diff, std::cmp::min(width_diff.abs(), height_diff.abs()))
        }).collect::<Vec<_>>();

        // look for kinds for which we would downscale tiles
        let downscaling_tile_kinds_data = tile_kinds_data.iter().filter(|(_, width_diff, height_diff, _)|
            std::cmp::min(*width_diff, *height_diff) <= 0
        ).collect::<Vec<_>>();

        let (tile_kind, width_diff, height_diff, _) = match downscaling_tile_kinds_data.len() {
            // all kinds would need to be upscaled, chose the kind for which the tiles would need to be upscaled the less
            0 => tile_kinds_data.iter().min_by_key(|(_, _, _, min_diff)| *min_diff).unwrap(),
            // exactly one kind match for which the tiles would need to be downscaled
            1 => downscaling_tile_kinds_data.first().unwrap(),
            // more than one kind match for which the tiles would need to be downscaled, chose the kind with the least downscaling
            _ => downscaling_tile_kinds_data.iter().min_by_key(|(_, _, _, min_diff)| *min_diff).unwrap(),
        };

        let mut tile_dimensions = tile_kind.dimensions();
        if width_diff < height_diff {
            // tile_dimensions.width = (tile_dimensions.width as i32 + width_diff) as u32;
            tile_dimensions.width = (tile_dimensions.width as i32 + width_diff).try_into().unwrap();
            tile_dimensions.height = tile_dimensions.height * tile_dimensions.width / tile_kind.dimensions().width;
        } else {
            // tile_dimensions.height = (tile_dimensions.height as i32 + height_diff) as u32;
            tile_dimensions.height = (tile_dimensions.height as i32 + height_diff).try_into().unwrap();
            tile_dimensions.width = tile_dimensions.width * tile_dimensions.height / tile_kind.dimensions().height;
        }

        let overlay_dimensions = self.dimensions_pixels_for_tile_dimensions(tile_dimensions);

        (*tile_kind, tile_dimensions, overlay_dimensions)
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
