

use hd_fpv_osd_font_tool::prelude::*;
use strum::IntoEnumIterator;

use super::Dimensions as OverlayFrameDimensions;

use crate::osd;
use crate::osd::dji::VideoResolutionTooSmallError;
use crate::video::resolution::Resolution as VideoResolution;


impl osd::Kind {

    pub fn dimensions_pixels_for_tile_kind(&self, tile_kind: tile::Kind) -> OverlayFrameDimensions {
        self.dimensions_tiles() * tile_kind.dimensions()
    }

    pub fn dimensions_pixels_for_tile_dimensions(&self, tile_dimensions: tile::Dimensions) -> OverlayFrameDimensions {
        self.dimensions_tiles() * tile_dimensions
    }

    pub fn dimensions_pixels(&self) -> OverlayFrameDimensions {
        self.dimensions_tiles() * self.tile_kind().dimensions()
    }

    /// Returns the best kind of tile to use without rescaling tiles so that the OSD fills as much as the screen as possible
    pub fn best_kind_of_tiles_to_use_without_scaling(&self, video_resolution: VideoResolution) -> Result<tile::Kind, VideoResolutionTooSmallError> {
        let avg_margins = tile::Kind::iter().flat_map(|tile_kind| {
            let osd_dimensions = self.dimensions_pixels_for_tile_kind(tile_kind);
            let (margin_width, margin_height) = crate::video::margins(video_resolution, osd_dimensions);
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

    pub fn best_kind_of_tiles_to_use_with_scaling(&self, max_resolution: OverlayFrameDimensions) -> (tile::Kind, tile::Dimensions, OverlayFrameDimensions) {
        let max_tile_width = max_resolution.width / self.dimensions_tiles().width;
        let max_tile_height = max_resolution.height / self.dimensions_tiles().height;
        let tile_kinds_data = tile::Kind::iter().map(|tile_kind| {
            let width_diff = max_tile_width as i32 - tile_kind.dimensions().width as i32;
            let height_diff = max_tile_height as i32 - tile_kind.dimensions().height as i32;
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
            tile_dimensions.width = (tile_dimensions.width as i32 + width_diff).try_into().unwrap();
            tile_dimensions.height = tile_dimensions.height * tile_dimensions.width / tile_kind.dimensions().width;
        } else {
            tile_dimensions.height = (tile_dimensions.height as i32 + height_diff).try_into().unwrap();
            tile_dimensions.width = tile_dimensions.width * tile_dimensions.height / tile_kind.dimensions().height;
        }

        let overlay_dimensions = self.dimensions_pixels_for_tile_dimensions(tile_dimensions);

        (*tile_kind, tile_dimensions, overlay_dimensions)
    }

}