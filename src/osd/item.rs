
use std::collections::HashMap;

use getset::{CopyGetters, Getters};
use strum::IntoEnumIterator;

use super::dji::file::FontVariant;
use super::dji::file::tile_indices::TileIndex;
use super::Dimensions;
use crate::osd;

#[derive(Debug, Clone, Copy, CopyGetters)]
#[getset(get_copy = "pub")]
pub struct Offset {
    x: i8,
    y: i8,
}

impl Offset {
    pub const fn new(x: i8, y: i8) -> Self { Self { x, y } }
}

#[derive(Getters, CopyGetters)]
#[getset(get_copy = "pub")]
pub struct LocationData {
    name: &'static str,
    marker_tile_indices: &'static [TileIndex],
    top_left_offset: Offset,
    dimensions: Dimensions
}

impl LocationData {

    pub const fn new(name: &'static str, marker_tile_indices: &'static [TileIndex], top_left_offset_x: i8, top_left_offset_y: i8, width: u8, height: u8) -> Self {
        Self {
            name,
            marker_tile_indices,
            top_left_offset: Offset::new(top_left_offset_x, top_left_offset_y),
            dimensions: Dimensions { width , height }
        }
    }

    pub fn region(&self, marker_coordinates: osd::Coordinates) -> osd::Region {
        let top_left_corner = osd::SignedCoordinates::new(
            (marker_coordinates.x as osd::SignedCoordinate).saturating_add(self.top_left_offset.x),
            (marker_coordinates.y as osd::SignedCoordinate).saturating_add(self.top_left_offset.y),
        );
        osd::Region::new(top_left_corner, self.dimensions)
    }

}

const fn ld(name: &'static str, marker_tile_indices: &'static [TileIndex], width: u8) -> LocationData {
    LocationData::new(name, marker_tile_indices, 0, 0, width, 1)
}

const fn ldo(name: &'static str, marker_tile_indices: &'static [TileIndex], top_left_offset_x: i8, width: u8) -> LocationData {
    LocationData::new(name, marker_tile_indices, top_left_offset_x, 0, width, 1)
}

#[allow(dead_code)]
const fn lde(name: &'static str, marker_tile_indices: &'static [TileIndex], top_left_offset_x: i8, top_left_offset_y: i8, width: u8, height: u8) -> LocationData {
    LocationData::new(name, marker_tile_indices, top_left_offset_x, top_left_offset_y, width, height)
}

mod location_data {
    use super::{LocationData, ld, ldo};

    pub const INAV: [LocationData; 3] = [
        ld("gpslat", &[3], 10),
        ld("gpslon", &[4], 10),
        ldo("alt", &[0x76, 0x77, 0x78, 0x79], -4, 5),
    ];

    pub const ARDUPILOT: [LocationData; 5] = [
        ld("gpslat", &[0xA6], 10),
        ld("gpslon", &[0xA7], 11),
        ldo("alt", &[0xB1, 0xB3], -4, 5),
        ldo("short+code", &[0x2B], -4, 8),
        ldo("long+code", &[0x2B], -8, 12),
    ];

}

impl FontVariant {
    pub const fn osd_items_location_data(&self) -> &'static [LocationData] {
        match self {
            FontVariant::Generic => &[],
            FontVariant::Ardupilot => &location_data::ARDUPILOT,
            FontVariant::Betaflight => &[],
            FontVariant::INAV => &location_data::INAV,
            FontVariant::KISSUltra => &[],
            FontVariant::Unknown => &[],
        }
    }

    pub fn find_osd_item_location_data(&self, item_name: &str) -> Option<&LocationData> {
        self.osd_items_location_data().iter().find(|ld| ld.name == item_name)
    }

    pub fn osd_item_names() -> HashMap<FontVariant, Vec<&'static str>> {
        let mut map = HashMap::default();
        for font_variant in Self::iter() {
            map.insert(font_variant, font_variant.osd_items_location_data().iter().map(LocationData::name).collect());
        }
        map
    }
}