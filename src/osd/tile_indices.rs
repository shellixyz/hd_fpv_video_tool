
use std::{iter::Enumerate, ops::Index};

use derive_more::Deref;
use thiserror::Error;

use crate::osd;

use super::{FontVariant, Dimensions, Kind};

pub type TileIndex = u16;

// frame payloads are always 1320*2=2640 bytes representing a 60x22 grid which corresponds to the FakeHD OSD format
pub const DIMENSIONS: Dimensions = Kind::DJI_FakeHD.dimensions_tiles();
pub const COUNT: usize = DIMENSIONS.width as usize * DIMENSIONS.height as usize;

#[derive(Debug, Error)]
#[error("unknown OSD item for `{font_variant}` font variant: {item_name}")]
pub struct UnknownOSDItem {
    font_variant: FontVariant,
    item_name: String,
}

impl UnknownOSDItem {
    pub fn new(font_variant: FontVariant, item_name: &str) -> Self { Self { font_variant, item_name: item_name.to_owned() } }
}

#[derive(Debug, Deref, Clone, PartialEq, Eq)]
pub struct TileIndices(Vec<TileIndex>);

impl TileIndices {

    pub fn new(inner: Vec<TileIndex>) -> Self {
        Self(inner)
    }

    fn screen_coordinates_to_index(x: osd::Coordinate, y: osd::Coordinate) -> usize {
        y as usize + x as usize * DIMENSIONS.height as usize
    }

    fn index_to_screen_coordinates(index: usize) -> osd::Coordinates {
        osd::Coordinates::new(
            (index / DIMENSIONS.height as usize) as osd::Coordinate,
            (index % DIMENSIONS.height as usize) as osd::Coordinate
        )
    }

    pub fn enumerate(&self) -> TileIndicesEnumeratorIter {
        TileIndicesEnumeratorIter(self.iter().enumerate())
    }

    fn enumerate_mut(&mut self) -> TileIndicesEnumeratorIterMut {
        TileIndicesEnumeratorIterMut(self.0.iter_mut().enumerate())
    }

    pub fn erase_region(&mut self, region: &osd::Region) {
        let coordinates_range = region.to_coordinates_range();
        for (coordinates, tile_index) in self.enumerate_mut() {
            if coordinates_range.contains(coordinates) {
                *tile_index = 0;
            }
        }
    }

    pub fn erase_regions(&mut self, regions: &[osd::Region]) {
        for region in regions {
            self.erase_region(region)
        }
    }

    pub fn erase_osd_item(&mut self, font_variant: FontVariant, item_name: impl AsRef<str>) -> Result<(), UnknownOSDItem> {
        let oild = font_variant.find_osd_item_location_data(item_name.as_ref())
            .ok_or_else(|| UnknownOSDItem::new(font_variant, item_name.as_ref()))?;

        let regions: Vec<osd::Region> = oild.marker_tile_indices().iter().flat_map(|marker_tile_index| {
            self.enumerate().filter_map(|(coordinates, tile_index)| {
                if tile_index == *marker_tile_index { Some(oild.region(coordinates)) } else { None }
            }).collect::<Vec<_>>()
        }).collect();

        self.erase_regions(&regions);
        Ok(())
    }

    pub fn erase_osd_items(&mut self, font_variant: FontVariant, item_names: &[impl AsRef<str>]) -> Result<(), UnknownOSDItem> {
        for item_name in item_names {
            self.erase_osd_item(font_variant, item_name)?;
        }
        Ok(())
    }

}

impl Index<(osd::Coordinate, osd::Coordinate)> for TileIndices {
    type Output = TileIndex;

    fn index(&self, index: (osd::Coordinate, osd::Coordinate)) -> &Self::Output {
        &self.0[Self::screen_coordinates_to_index(index.0, index.1)]
    }
}

pub struct TileIndicesEnumeratorIter<'a>(Enumerate<std::slice::Iter<'a, u16>>);

impl<'a> Iterator for TileIndicesEnumeratorIter<'a> {
    type Item = (osd::Coordinates, TileIndex);

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
    type Item = (osd::Coordinates, &'a mut TileIndex);

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