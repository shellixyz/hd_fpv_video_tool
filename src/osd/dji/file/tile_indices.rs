
use std::{iter::Enumerate, ops::Index};

use derive_more::Deref;

use crate::osd::dji::{Kind, Dimensions};


pub type TileIndex = u16;
pub type ScreenCoordinate = u8;

// frame payloads are always 1320*2=2640 bytes representing a 60x22 grid which corresponds to the FakeHD OSD format
pub const TILE_INDICES_DIMENSIONS_TILES: Dimensions = Kind::FakeHD.dimensions_tiles();

#[derive(Debug, Deref, Clone, PartialEq, Eq)]
pub struct TileIndices(Vec<TileIndex>);

impl TileIndices {

    pub fn new(inner: Vec<TileIndex>) -> Self {
        Self(inner)
    }

    fn screen_coordinates_to_index(x: ScreenCoordinate, y: ScreenCoordinate) -> usize {
        y as usize + x as usize * TILE_INDICES_DIMENSIONS_TILES.height as usize
    }

    fn index_to_screen_coordinates(index: usize) -> (ScreenCoordinate, ScreenCoordinate) {
        (
            (index / TILE_INDICES_DIMENSIONS_TILES.height as usize) as ScreenCoordinate,
            (index % TILE_INDICES_DIMENSIONS_TILES.height as usize) as ScreenCoordinate
        )
    }

    pub fn enumerate(&self) -> TileIndicesEnumeratorIter {
        TileIndicesEnumeratorIter(self.iter().enumerate())
    }

}

impl Index<(ScreenCoordinate, ScreenCoordinate)> for TileIndices {
    type Output = TileIndex;

    fn index(&self, index: (ScreenCoordinate, ScreenCoordinate)) -> &Self::Output {
        &self.0[Self::screen_coordinates_to_index(index.0, index.1)]
    }
}

pub struct TileIndicesEnumeratorIter<'a>(Enumerate<std::slice::Iter<'a, u16>>);

impl<'a> Iterator for TileIndicesEnumeratorIter<'a> {
    type Item = (ScreenCoordinate, ScreenCoordinate, TileIndex);

    fn next(&mut self) -> Option<Self::Item> {
        for (tile_index_index, tile_index) in self.0.by_ref() {
            if *tile_index > 0 {
                let (screen_x, screen_y) = TileIndices::index_to_screen_coordinates(tile_index_index);
                return Some((screen_x, screen_y, *tile_index))
            }
        }
        None
    }
}
