
use hd_fpv_osd_font_tool::osd::tile::{self, Dimensions, Tile};
use indicatif::{ParallelProgressIterator, ProgressStyle};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};


pub trait ResizeTiles {
    fn resized_tiles_par_with_progress(&self, new_dimensions: Dimensions) -> Vec<tile::Image>;
}

impl ResizeTiles for &[Tile]
{
    fn resized_tiles_par_with_progress(&self, new_dimensions: Dimensions) -> Vec<tile::Image> {
        let tile_dimensions = self.first().unwrap().dimensions();
        log::info!("resizing {} tiles from {}x{} to {new_dimensions}", self.len(), tile_dimensions.0, tile_dimensions.1);
        let progress_style = ProgressStyle::with_template("{wide_bar} {pos:>6}/{len}").unwrap();
        self.par_iter().progress_with_style(progress_style).map(|tile|
            image::imageops::resize(tile.image(), new_dimensions.width, new_dimensions.height, image::imageops::FilterType::Lanczos3)
        ).collect()
    }
}