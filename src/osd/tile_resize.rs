
use hd_fpv_osd_font_tool::osd::tile::{self, Dimensions, Tile};

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
    fn resize_tiles(&self, new_dimensions: Dimensions) -> Vec<tile::Image>;
}

impl<T> ResizeTiles for T
where
    for<'any> &'any T: IntoIterator<Item = &'any Tile>,
{
    fn resize_tiles(&self, new_dimensions: Dimensions) -> Vec<tile::Image> {
        self.into_iter().map(|tile|
            image::imageops::resize(tile.image(), new_dimensions.width, new_dimensions.height, image::imageops::FilterType::Lanczos3)
        ).collect()
    }
}