
use hd_fpv_osd_font_tool::osd::tile;

pub trait TileImage {
    fn image(&self) -> &tile::Image;
}

pub struct ResizedTile(tile::Image);

impl ResizedTile {
    pub fn new(image: tile::Image) -> Self {
        Self(image)
    }
}

impl TileImage for ResizedTile {
    fn image(&self) -> &tile::Image {
        &self.0
    }
}

impl TileImage for tile::Tile {
    fn image(&self) -> &tile::Image {
        self.image()
    }
}