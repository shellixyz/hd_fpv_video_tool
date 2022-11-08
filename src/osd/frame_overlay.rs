
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::{fmt::Display, error::Error};
use std::io::Error as IOError;

use super::file::{Frame as OSDFileFrame, ReadError};
use super::file::FrameIndex;
use super::file::Reader as OSDFileReader;

use derive_more::{From, Error};
use hd_fpv_osd_font_tool::osd::tile::container::{UniqTileKind, TileKindError};
use image::{ImageBuffer, Rgba, GenericImage, ImageError};
use hd_fpv_osd_font_tool::osd::tile::{self, Tile};
use hd_fpv_osd_font_tool::dimensions::Dimensions;
use indicatif::{ProgressStyle, ParallelProgressIterator};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use strum::Display;

pub type DimensionsTiles = Dimensions<u32>;

#[derive(Debug)]
pub struct InvalidDimensionsTilesError(pub DimensionsTiles);
impl Error for InvalidDimensionsTilesError {}

impl Display for InvalidDimensionsTilesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid dimensions tiles: {}x{}", self.0.width(), self.0.height())
    }
}

#[derive(Debug, From)]
pub enum DrawFrameOverlayError {
    InvalidFontTileKindForOverlayKind {
        needed_font_tile_kind: tile::Kind,
        got_font_tile_kind: tile::Kind,
        overlay_kind: Kind
    },
    TileKindError(TileKindError),
}

impl Error for DrawFrameOverlayError {}

impl Display for DrawFrameOverlayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DrawFrameOverlayError::InvalidFontTileKindForOverlayKind { needed_font_tile_kind, got_font_tile_kind, overlay_kind } =>
                write!(f, "invalid font tile kind for {} overlay kind: need {} font tile kind, got {}", overlay_kind, needed_font_tile_kind, got_font_tile_kind),
            DrawFrameOverlayError::TileKindError(error) => error.fmt(f),
        }
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
        (dimensions_tiles.width * tile_dimensions.width, dimensions_tiles.height * tile_dimensions.height)

    }

}

impl TryFrom<&DimensionsTiles> for Kind {
    type Error = InvalidDimensionsTilesError;

    fn try_from(dimensions_tiles: &DimensionsTiles) -> Result<Self, Self::Error> {
        match *dimensions_tiles {
            dimensions_tiles::SD => Ok(Self::SD),
            dimensions_tiles::FAKE_HD => Ok(Self::FakeHD),
            dimensions_tiles::HD => Ok(Self::HD),
            _ => Err(InvalidDimensionsTilesError(*dimensions_tiles))
        }
    }
}

pub type Image = ImageBuffer<Rgba<u8>, Vec<u8>>;

pub fn transparent_frame_overlay(kind: &Kind) -> Image {
    let (image_width, image_height) = kind.dimensions_pixels();
    Image::new(image_width, image_height)
}

pub fn draw_frame_overlay(kind: &Kind, osd_file_frame: &OSDFileFrame, font_tiles: &Vec<Tile>) -> Result<Image, DrawFrameOverlayError> {
    let font_tiles_kind = font_tiles.tile_kind()?;
    if kind.tile_kind() != font_tiles_kind {
        return Err(DrawFrameOverlayError::InvalidFontTileKindForOverlayKind { needed_font_tile_kind: kind.tile_kind(), got_font_tile_kind: font_tiles_kind, overlay_kind: *kind });
    }
    let tile_dimensions = kind.tile_kind().dimensions();
    let mut image = transparent_frame_overlay(kind);
    for (screen_x, screen_y, tile_index) in osd_file_frame.enumerate_tile_indices() {
        image.copy_from(font_tiles[tile_index as usize].image(), screen_x as u32 * tile_dimensions.width, screen_y as u32 * tile_dimensions.height).unwrap();
    }
    Ok(image)
}

pub fn format_overlay_frame_file_index(frame_index: FrameIndex) -> String {
    format!("{:010}.png", frame_index)
}

pub fn make_overlay_frame_file_path<P: AsRef<Path>>(dir_path: P, frame_index: FrameIndex) -> PathBuf {
    [dir_path.as_ref().to_str().unwrap(), &format_overlay_frame_file_index(frame_index)].iter().collect()
}

pub fn link_missing_frames<P: AsRef<Path>>(dir_path: P, existing_frame_indices: &BTreeSet<FrameIndex>) -> Result<(), IOError> {
    let existing_frame_indices_vec = existing_frame_indices.iter().collect::<Vec<&FrameIndex>>();
    for indices in existing_frame_indices_vec.windows(2) {
        if let &[lower_index, greater_index] = indices {
            if *greater_index > lower_index + 1 {
                let original_path = make_overlay_frame_file_path(&dir_path, *lower_index);
                for link_to_index in lower_index+1..*greater_index {
                    let copy_path = make_overlay_frame_file_path(&dir_path, link_to_index);
                    #[allow(clippy::needless_borrow)]
                    std::fs::hard_link(&original_path, copy_path)?;
                }
            }
        }
    }
    Ok(())
}

#[derive(Debug, Display, Error, From)]
pub enum SaveFramesToDirError {
    IOError(IOError),
    ReadError(ReadError),
    ImageError(ImageError),
}

pub struct Generator<'a> {
    reader: OSDFileReader,
    font_tiles: &'a Vec<Tile>,
}

impl<'a> Generator<'a> {

    pub fn new(reader: OSDFileReader, font_tiles: &'a Vec<Tile>) -> Result<Self, DrawFrameOverlayError> {
        let font_tiles_kind = font_tiles.tile_kind()?;
        let overlay_kind = reader.overlay_kind();
        if overlay_kind.tile_kind() != font_tiles_kind {
            return Err(
                DrawFrameOverlayError::InvalidFontTileKindForOverlayKind {
                    needed_font_tile_kind: overlay_kind.tile_kind(),
                    got_font_tile_kind: font_tiles_kind, overlay_kind: *overlay_kind
                }
            );
        }
        Ok(Self { reader, font_tiles })
    }

    pub fn draw_next_frame(&mut self) -> Result<Option<Image>, ReadError> {
        match self.reader.read_frame()? {
            Some(frame) => Ok(Some(draw_frame_overlay(self.reader.overlay_kind(), &frame, self.font_tiles).unwrap())),
            None => Ok(None),
        }
    }

    pub fn save_frames_to_dir<P: AsRef<Path> + Display + std::marker::Sync>(&mut self, path: P, frame_offset: i32) -> Result<(), SaveFramesToDirError> {
        std::fs::create_dir_all(&path)?;
        log::info!("generating overlay frames and saving into directory: {path}");
        let frames = self.reader.frames()?;
        let overlay_kind = self.reader.overlay_kind();

        let first_frame_index = frames.iter().position(|frame| (*frame.index() as i32) > -frame_offset).unwrap();
        let frames = &frames[first_frame_index..];
        let first_frame_index = frames.first().unwrap().index();

        let missing_frames = frame_offset + *first_frame_index as i32;

        // we are missing frames at the beginning
        if missing_frames > 0 {
            log::debug!("Generating blank frames 0..{}", missing_frames - 1);
            let frame_0_path = make_overlay_frame_file_path(&path, 0);
            transparent_frame_overlay(overlay_kind).save(&frame_0_path)?;
            for frame_index in 1..missing_frames {
                std::fs::hard_link(&frame_0_path, make_overlay_frame_file_path(&path, frame_index as FrameIndex))?;
            }
        }

        let frame_count = *frames.last().unwrap().index();
        let progress_style = ProgressStyle::with_template("{wide_bar} {pos:>6}/{len}").unwrap();
        frames.par_iter().progress_with_style(progress_style).try_for_each(|frame| {
            let actual_frame_index = (*frame.index() as i32 + frame_offset) as u32;
            log::debug!("{} -> {}", frame.index(), &actual_frame_index);
            let frame_image = draw_frame_overlay(overlay_kind, frame, self.font_tiles).unwrap();
            frame_image.save(make_overlay_frame_file_path(&path, actual_frame_index))
        })?;

        log::info!("linking missing overlay frames");
        let frame_indices = frames.iter().map(|x| (*x.index() as i32 + frame_offset) as u32).collect();
        link_missing_frames(&path, &frame_indices)?;

        log::info!("overlay frames generation completed: {} frames", frame_count);
        Ok(())
    }

}