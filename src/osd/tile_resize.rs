
use hd_fpv_osd_font_tool::osd::tile::{self, Dimensions, Tile};
use indicatif::{ParallelProgressIterator, ProgressStyle};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

// use super::tile::ResizedTile;

// pub trait ResizeTiles {
//     fn resize_tiles(&self, new_dimensions: Dimensions) -> Vec<ResizedTile>;
// }

// impl<T> ResizeTiles for T
// where
//     for<'any> &'any T: IntoIterator<Item = &'any Tile>,
// {
//     fn resize_tiles(&self, new_dimensions: Dimensions) -> Vec<ResizedTile> {
//         self.into_iter().map(|tile|
//             ResizedTile::new(image::imageops::resize(tile.image(), new_dimensions.width, new_dimensions.height, image::imageops::FilterType::Lanczos3))
//         ).collect()
//     }
// }

pub trait ResizeTiles {
    fn resized_tiles_par_with_progress(&self, new_dimensions: Dimensions) -> Vec<tile::Image>;
}

// impl<T> ResizeTiles for T
// where
//     for<'any> &'any T: IntoIterator<Item = &'any Tile>,
// {
//     fn par_resize_tiles_with_progress(&self, new_dimensions: Dimensions) -> Vec<tile::Image> {
//         self.into_iter().map(|tile|
//             image::imageops::resize(tile.image(), new_dimensions.width, new_dimensions.height, image::imageops::FilterType::Lanczos3)
//         ).collect()
//     }
// }

impl ResizeTiles for &[Tile]
{
    fn resized_tiles_par_with_progress(&self, new_dimensions: Dimensions) -> Vec<tile::Image> {
        let tile_dimensions = self.first().unwrap().dimensions();
        log::info!("Resizing {} tiles from {}x{} to {new_dimensions}", self.len(), tile_dimensions.0, tile_dimensions.1);
        let progress_style = ProgressStyle::with_template("{wide_bar} {pos:>6}/{len}").unwrap();
        self.par_iter().progress_with_style(progress_style).map(|tile|
            image::imageops::resize(tile.image(), new_dimensions.width, new_dimensions.height, image::imageops::FilterType::Lanczos3)
        ).collect()
    }
}