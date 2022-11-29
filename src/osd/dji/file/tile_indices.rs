
use std::{iter::Enumerate, ops::Index};

use derive_more::Deref;

use crate::osd::dji::{Kind, Dimensions};
use crate::prelude::*;


pub type TileIndex = u16;

// frame payloads are always 1320*2=2640 bytes representing a 60x22 grid which corresponds to the FakeHD OSD format
pub const TILE_INDICES_DIMENSIONS_TILES: Dimensions = Kind::FakeHD.dimensions_tiles();

#[derive(Debug, Deref, Clone, PartialEq, Eq)]
pub struct TileIndices(Vec<TileIndex>);

impl TileIndices {

    pub fn new(inner: Vec<TileIndex>) -> Self {
        Self(inner)
    }

    fn screen_coordinates_to_index(x: OSDCoordinate, y: OSDCoordinate) -> usize {
        y as usize + x as usize * TILE_INDICES_DIMENSIONS_TILES.height as usize
    }

    fn index_to_screen_coordinates(index: usize) -> OSDCoordinates {
        OSDCoordinates::new(
            (index / TILE_INDICES_DIMENSIONS_TILES.height as usize) as OSDCoordinate,
            (index % TILE_INDICES_DIMENSIONS_TILES.height as usize) as OSDCoordinate
        )
    }

    pub fn enumerate(&self) -> TileIndicesEnumeratorIter {
        TileIndicesEnumeratorIter(self.iter().enumerate())
    }

    fn enumerate_mut(&mut self) -> TileIndicesEnumeratorIterMut {
        TileIndicesEnumeratorIterMut(self.0.iter_mut().enumerate())
    }

    pub fn erase_region(&mut self, region: &OSDRegion) {
        let coordinates_range = region.to_coordinates_range();
        for (coordinates, tile_index) in self.enumerate_mut() {
            if coordinates_range.contains(&coordinates) {
                *tile_index = 0;
            }
        }
    }

    pub fn erase_regions(&mut self, regions: &[OSDRegion]) {
        for region in regions {
            self.erase_region(region)
        }
    }

}

impl Index<(OSDCoordinate, OSDCoordinate)> for TileIndices {
    type Output = TileIndex;

    fn index(&self, index: (OSDCoordinate, OSDCoordinate)) -> &Self::Output {
        &self.0[Self::screen_coordinates_to_index(index.0, index.1)]
    }
}

pub struct TileIndicesEnumeratorIter<'a>(Enumerate<std::slice::Iter<'a, u16>>);

impl<'a> Iterator for TileIndicesEnumeratorIter<'a> {
    type Item = (OSDCoordinates, TileIndex);

    fn next(&mut self) -> Option<Self::Item> {
        for (tile_index_index, tile_index) in self.0.by_ref() {
            if *tile_index > 0 {
                let coordinates = TileIndices::index_to_screen_coordinates(tile_index_index);
                return Some((coordinates, *tile_index))
            }
        }
        None
    }
}

struct TileIndicesEnumeratorIterMut<'a>(Enumerate<std::slice::IterMut<'a, u16>>);

impl<'a> Iterator for TileIndicesEnumeratorIterMut<'a> {
    type Item = (OSDCoordinates, &'a mut TileIndex);

    fn next(&mut self) -> Option<Self::Item> {
        for (tile_index_index, tile_index) in self.0.by_ref() {
            if *tile_index > 0 {
                let coordinates = TileIndices::index_to_screen_coordinates(tile_index_index);
                return Some((coordinates, tile_index))
            }
        }
        None
    }
}