
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::{fmt::Display, error::Error};
use std::io::Error as IOError;

use super::file::Frame as OSDFileFrame;
use super::file::FrameIndex as OSDFileFrameIndex;

use getset::Getters;
use image::{ImageBuffer, Rgba, GenericImage};
use hd_fpv_osd_font_tool::osd::{standard_size_tile_container::StandardSizeTileArray, tile};
use strum::Display;

#[derive(Debug)]
pub struct InvalidDimensionsTilesError(pub DimensionsTiles);
impl Error for InvalidDimensionsTilesError {}

impl Display for InvalidDimensionsTilesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid dimensions tiles: {}x{}", self.0.width(), self.0.height())
    }
}

#[derive(Debug)]
pub enum DrawFrameOverlayError {
    InvalidFontTileKindForOverlayKind {
        needed_font_tile_kind: tile::Kind,
        got_font_tile_kind: tile::Kind,
        overlay_kind: Kind
    }
}

impl Error for DrawFrameOverlayError {}

impl Display for DrawFrameOverlayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DrawFrameOverlayError::InvalidFontTileKindForOverlayKind { needed_font_tile_kind, got_font_tile_kind, overlay_kind } =>
                write!(f, "invalid font tile kind for {} overlay kind: need {} font tile kind, got {}", overlay_kind, needed_font_tile_kind, got_font_tile_kind)
        }
    }
}

#[derive(Debug, Getters, PartialEq, Eq, Clone)]
#[getset(get = "pub")]
pub struct DimensionsTiles {
    width: u8,
    height: u8
}

impl DimensionsTiles {
    pub const fn new(width: u8, height: u8) -> Self {
        Self { width, height }
    }
}

pub mod dimensions_tiles {
    use super::DimensionsTiles;
    pub const SD: DimensionsTiles = DimensionsTiles::new(30, 15);
    pub const FAKE_HD: DimensionsTiles = DimensionsTiles::new(60, 22);
    pub const HD: DimensionsTiles = DimensionsTiles::new(50, 18);
}

#[derive(Debug, Display, Clone, Copy)]
pub enum Kind {
    SD,
    FakeHD,
    HD
}

impl Kind {

    pub const fn dimensions_tiles(&self) -> DimensionsTiles {
        use Kind::*;
        match self {
            SD => dimensions_tiles::SD,
            FakeHD => dimensions_tiles::FAKE_HD,
            HD => dimensions_tiles::HD,
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

    pub const fn dimensions_pixels(&self) -> (u32, u32) {
        let dimensions_tiles = self.dimensions_tiles();
        let tile_dimensions = self.tile_kind().dimensions();
        (dimensions_tiles.width as u32 * tile_dimensions.width, dimensions_tiles.height as u32 * tile_dimensions.height)

    }

}

impl TryFrom<&DimensionsTiles> for Kind {
    type Error = InvalidDimensionsTilesError;

    fn try_from(dimensions_tiles: &DimensionsTiles) -> Result<Self, Self::Error> {
        match *dimensions_tiles {
            dimensions_tiles::SD => Ok(Self::SD),
            dimensions_tiles::FAKE_HD => Ok(Self::FakeHD),
            dimensions_tiles::HD => Ok(Self::HD),
            _ => Err(InvalidDimensionsTilesError(dimensions_tiles.clone()))
        }
    }
}

pub type Image = ImageBuffer<Rgba<u8>, Vec<u8>>;

pub fn transparent_frame_overlay(kind: &Kind) -> Image {
    let (image_width, image_height) = kind.dimensions_pixels();
    Image::new(image_width, image_height)
}

pub fn draw_frame_overlay(kind: &Kind, osd_file_frame: &OSDFileFrame, font_tiles: &StandardSizeTileArray) -> Result<Image, DrawFrameOverlayError> {
    if kind.tile_kind() != font_tiles.tile_kind() {
        return Err(DrawFrameOverlayError::InvalidFontTileKindForOverlayKind { needed_font_tile_kind: kind.tile_kind(), got_font_tile_kind: font_tiles.tile_kind(), overlay_kind: *kind });
    }
    let tile_dimensions = kind.tile_kind().dimensions();
    let mut image = transparent_frame_overlay(kind);
    for (screen_x, screen_y, tile_index) in osd_file_frame.enumerate_tile_indices() {
        image.copy_from(font_tiles[tile_index as usize].image(), screen_x as u32 * tile_dimensions.width, screen_y as u32 * tile_dimensions.height).unwrap();
    }
    Ok(image)
}

pub fn format_overlay_frame_file_index(frame_index: OSDFileFrameIndex) -> String {
    format!("{:010}.png", frame_index)
}

pub fn make_overlay_frame_file_path<P: AsRef<Path>>(dir_path: P, frame_index: OSDFileFrameIndex) -> PathBuf {
    [dir_path.as_ref().to_str().unwrap(), &format_overlay_frame_file_index(frame_index)].iter().collect()
}

pub fn link_missing_frames<P: AsRef<Path>>(dir_path: P, existing_frame_indices: &BTreeSet<OSDFileFrameIndex>) -> Result<(), IOError> {
    let existing_frame_indices_vec = existing_frame_indices.iter().collect::<Vec<&OSDFileFrameIndex>>();
    for indices in existing_frame_indices_vec.windows(2) {
        if let &[lower_index, greater_index] = indices {
            if *greater_index > lower_index + 1 {
                let original_path = make_overlay_frame_file_path(&dir_path, *lower_index);
                for link_to_index in lower_index+1..*greater_index {
                    let copy_path = make_overlay_frame_file_path(&dir_path, link_to_index);
                    std::fs::hard_link(&original_path, copy_path)?;
                }
            }
        }
    }
    Ok(())
}