
use std::{
    collections::BTreeSet,
    path::{
        Path,
        PathBuf
    },
    io::Error as IOError,
    fmt::Display,
};

use clap::Args;
use derive_more::From;
use getset::Getters;
use regex::Regex;
use strum::EnumIter;
use strum::IntoEnumIterator;
use thiserror::Error;
use image::{ImageBuffer, Rgba, GenericImage};
use indicatif::{ProgressStyle, ParallelProgressIterator};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use lazy_static::lazy_static;

use hd_fpv_osd_font_tool::{
    prelude::*,
    dimensions::Dimensions as GenericDimensions,
};

use crate::{
    create_path::{
        CreatePathError,
        create_path
    },
    file::{
        self,
        HardLinkError
    },
    image::{
        WriteImageFile,
        WriteError as ImageWriteError,
    },
};

use super::{
    dji::{
        Kind as DJIOSDKind,
        VideoResolutionTooSmallError,
        font_dir::FontDir,
        file::{
            Frame as OSDFileFrame,
            FrameIndex,
            ReadError,
            Reader as OSDFileReader,
        },
        utils,
    },
    tile_resize::ResizeTiles,
};


pub type VideoResolution = GenericDimensions<u32>;
pub type Resolution = GenericDimensions<u32>;

#[derive(Debug, Error, From)]
pub enum DrawFrameOverlayError {
    #[error(transparent)]
    ReadError(ReadError),
    #[error("failed to load font file: {0}")]
    FontLoadError(bin_file::LoadError),
    #[error("video resolution {video_resolution} too small to render {osd_kind} OSD kind without scaling")]
    VideoResolutionTooSmallError{ osd_kind: DJIOSDKind, video_resolution: VideoResolution },
}

pub type Image = ImageBuffer<Rgba<u8>, Vec<u8>>;

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

#[derive(Debug, Error, From)]
pub enum SaveFramesToDirError {
    #[error(transparent)]
    CreatePathError(CreatePathError),
    #[error(transparent)]
    IOError(IOError),
    #[error(transparent)]
    ReadError(ReadError),
    #[error(transparent)]
    ImageWriteError(ImageWriteError),
    #[error(transparent)]
    HardLinkError(HardLinkError),
    #[error("target directory exists")]
    TargetDirectoryExists,
}

#[derive(Debug, Clone, Copy)]
pub enum Scaling {
    No {
        target_resolution: Option<TargetResolution>,
    },
    Yes {
        target_resolution: TargetResolution,
        min_margins: Margins,
    },
    Auto {
        target_resolution: TargetResolution,
        min_margins: Margins,
        min_resolution: Resolution,
    }
}

#[derive(Debug, Error, From)]
pub enum ScalingArgsError {
    #[error(transparent)]
    InvalidMarginsFormatError(InvalidMarginsFormatError),
    #[error("invalid minimum coverage percentage value: {0}")]
    InvalidMinCoveragePercent(u8),
    #[error("scaling and no-scaling arguments are mutually exclusive")]
    IncompatibleArguments,
    #[error("need target video resolution when scaling requested")]
    NeedTargetVideoResolution,
    #[error(transparent)]
    InvalidResolutionFormat(InvalidTargetResolutionError)
}

#[derive(Args)]
pub struct ScalingArgs {

    // TODO: try to generate list of valid values at run time so that it is always in sync with the TargetResolution enum
    /// valid values are 720p, 720p4:3, 1080p, 1080p4:3 or a custom resolution in the format <width>x<height>
    #[clap(short = 'r', long, value_parser = target_resolution_parser)]
    target_resolution: Option<TargetResolution>,

    /// force using scaling, default is automatic
    #[clap(short, long, value_parser)]
    scaling: bool,

    /// force disable scaling, default is automatic
    #[clap(short, long, value_parser)]
    no_scaling: bool,

    /// minimum margins to decide whether scaling should be used and how much to scale
    #[clap(long, value_parser, value_name = "horizontal:vertical", default_value = "20:20")]
    min_margins: String,

    /// minimum percentage of OSD coverage under which scaling will be used if --scaling/--no-scaling options are not provided
    #[clap(long, value_parser = clap::value_parser!(u8).range(1..=100), value_name = "percent", default_value = "90")]
    min_coverage: u8,
}

fn target_resolution_parser(target_resolution_str: &str) -> Result<TargetResolution, InvalidTargetResolutionError> {
    TargetResolution::try_from(target_resolution_str)
}

impl Scaling {
    pub fn try_from(args: &ScalingArgs) -> Result<Self, ScalingArgsError> {
        let min_margins = Margins::try_from(args.min_margins.as_str())?;
        Ok(match (args.scaling, args.no_scaling) {
            (true, true) => return Err(ScalingArgsError::IncompatibleArguments),
            (true, false) => {
                // let target_resolution_str = args.target_resolution.ok_or(ScalingArgsError::NeedTargetVideoResolution)?;
                // let target_resolution = TargetResolution::try_from(target_resolution_str.as_str())?;
                let target_resolution = args.target_resolution.ok_or(ScalingArgsError::NeedTargetVideoResolution)?;
                Scaling::Yes { target_resolution, min_margins }
            },
            (false, true) => Scaling::No { target_resolution: args.target_resolution },
            (false, false) => {
                if let Some(target_resolution) = args.target_resolution {
                    let min_coverage = args.min_coverage as f64 / 100.0;
                    let min_resolution = Resolution::new(
                        (target_resolution.dimensions().width as f64 * min_coverage) as u32,
                        (target_resolution.dimensions().height as f64 * min_coverage) as u32
                    );
                    Scaling::Auto { target_resolution, min_margins, min_resolution }
                } else {
                    Scaling::No { target_resolution: args.target_resolution }
                }
            },
        })
    }
}

#[derive(Debug, Error)]
#[error("invalid margins format: {0}")]
pub struct InvalidMarginsFormatError(String);

#[derive(Debug, Clone, Copy, Getters)]
#[getset(get_copy = "pub")]
pub struct Margins {
    horizontal: u32,
    vertical: u32,
}

impl TryFrom<&str> for Margins {
    type Error = InvalidMarginsFormatError;

    fn try_from(margins_str: &str) -> Result<Self, Self::Error> {
        lazy_static! {
            static ref MARGINS_RE: Regex = Regex::new(r"\A(?P<horiz>\d{1,3}):(?P<vert>\d{1,3})\z").unwrap();
        }
        match MARGINS_RE.captures(margins_str) {
            Some(captures) => {
                let horizontal = captures.name("horiz").unwrap().as_str().parse().unwrap();
                let vertical = captures.name("vert").unwrap().as_str().parse().unwrap();
                Ok(Self { horizontal, vertical })
            },
            None => Err(InvalidMarginsFormatError(margins_str.to_owned())),
        }
    }
}

#[derive(Debug, Clone, Copy, EnumIter)]
pub enum StandardResolution {
    Tr720p,
    Tr720p4By3,
    Tr1080p,
    Tr1080p4by3,
}

impl Display for StandardResolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use StandardResolution::*;
        let value_str = match self {
             Tr720p => "720p",
             Tr720p4By3 => "720p4:3",
             Tr1080p => "1080p",
             Tr1080p4by3 => "1080p4:3",
        };
        f.write_str(value_str)
    }
}

impl StandardResolution {
    pub fn list() -> String {
        Self::iter().map(|std_res|
            std_res.to_string()
        ).collect::<Vec<_>>().join(", ")
    }

    pub fn dimensions(&self) -> VideoResolution {
        use StandardResolution::*;
        match self {
            Tr720p => VideoResolution::new(1280, 720),
            Tr720p4By3 => VideoResolution::new(960, 720),
            Tr1080p => VideoResolution::new(1920, 1080),
            Tr1080p4by3 => VideoResolution::new(1440, 1080),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TargetResolution {
    Standard(StandardResolution),
    Custom(VideoResolution),
}

impl TargetResolution {
    pub fn dimensions(&self) -> VideoResolution {
        use TargetResolution::*;
        match self {
            Standard(std_res) => std_res.dimensions(),
            Custom(resolution) => *resolution,
        }
    }

    pub fn valid_list() -> String {
        [StandardResolution::list(), "<width>x<height>".to_owned()].join(", ")
    }
}

#[derive(Debug, Error)]
#[error("invalid target resolution `{given}`, valid resolutions are: {valid}")]
pub struct InvalidTargetResolutionError {
    given: String,
    valid: String
}

impl TryFrom<&str> for TargetResolution {
    type Error = InvalidTargetResolutionError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        use TargetResolution::*;
        let resolution = match value {
            "720p" => Standard(StandardResolution::Tr720p),
            "720p4:3" => Standard(StandardResolution::Tr1080p4by3),
            "1080p" => Standard(StandardResolution::Tr1080p),
            "1080p4:3" => Standard(StandardResolution::Tr1080p4by3),
            custom_res_str => {
                lazy_static! {
                    static ref RES_RE: Regex = Regex::new(r"\A(?P<width>\d{1,5})x(?P<height>\d{1,5})\z").unwrap();
                }
                match RES_RE.captures(custom_res_str) {
                    Some(captures) => {
                        let width = captures.name("width").unwrap().as_str().parse().unwrap();
                        let height = captures.name("height").unwrap().as_str().parse().unwrap();
                        Custom(VideoResolution::new(width, height))
                    },
                    None =>
                        return Err(InvalidTargetResolutionError {
                            given: custom_res_str.to_owned(),
                            valid: Self::valid_list()
                        }),
                }
            }
        };
        Ok(resolution)
    }
}

pub struct Generator {
    reader: OSDFileReader,
    tile_images: Vec<tile::Image>,
    overlay_resolution: VideoResolution,
}

impl Generator {

    fn best_settings_for_requested_scaling(osd_kind: DJIOSDKind, scaling: &Scaling) -> Result<(Resolution, tile::Kind, Option<TileDimensions>), DrawFrameOverlayError> {
        Ok(match *scaling {

            Scaling::No { target_resolution } => {
                match target_resolution {

                    // no scaling requested but target resolution provided: use the tile kind best matching the target resolution
                    Some(target_resolution) => {
                        let tile_kind = osd_kind.best_kind_of_tiles_to_use_without_scaling(target_resolution.dimensions()).map_err(|error| {
                            let VideoResolutionTooSmallError { osd_kind, video_resolution } = error;
                            DrawFrameOverlayError::VideoResolutionTooSmallError { osd_kind, video_resolution }
                        })?;
                        (osd_kind.dimensions_pixels_for_tile_kind(tile_kind), tile_kind, None)
                    },

                    // no target resolution specified so use the native tile kind for the OSD kind
                    None => (osd_kind.dimensions_pixels(), osd_kind.tile_kind(), None)

                }
            },

            Scaling::Yes { min_margins, target_resolution } => {
                let max_resolution = VideoResolution::new(
                    target_resolution.dimensions().width - 2 * min_margins.horizontal,
                    target_resolution.dimensions().height - 2 * min_margins.vertical,
                );
                let (tile_kind, tile_dimensions, overlay_dimensions) = osd_kind.best_kind_of_tiles_to_use_with_scaling(max_resolution);
                (overlay_dimensions, tile_kind, Some(tile_dimensions))
            },

            Scaling::Auto { min_margins, min_resolution, target_resolution } => {
                let (overlay_resolution, tile_kind, tile_scaling) =

                    // check results without scaling
                    match Self::best_settings_for_requested_scaling(osd_kind, &Scaling::No { target_resolution: Some(target_resolution) }) {

                        // no scaling is possible
                        Ok(values) => {
                            let (overlay_dimensions, _, _) = values;
                            let (margin_width, margin_height) = utils::margins(target_resolution.dimensions(), overlay_dimensions);
                            let min_margins_condition_met = margin_width >= min_margins.horizontal as i32 && margin_height >= min_margins.vertical as i32;
                            let min_dimensions_condition_met = overlay_dimensions.width >= min_resolution.width && overlay_dimensions.height >= min_resolution.height;

                            // check whether the result would match the user specified conditions
                            if min_margins_condition_met && min_dimensions_condition_met {
                                values
                            } else {
                                // else return parameters with scaling enabled
                                Self::best_settings_for_requested_scaling(osd_kind, &Scaling::Yes { target_resolution, min_margins })?
                            }

                        },

                        // no scaling does not work, return parameters with scaling enabled
                        Err(_) => Self::best_settings_for_requested_scaling(osd_kind, &Scaling::Yes { target_resolution, min_margins })?,
                    };

                let tile_scaling_yes_no = match tile_scaling { Some(_) => "yes", None => "no" };
                log::info!("calculated best approach: tile kind: {tile_kind} - scaling {tile_scaling_yes_no} - overlay resolution {overlay_resolution}");

                (overlay_resolution, tile_kind, tile_scaling)
            },
        })
    }

    pub fn new(mut reader: OSDFileReader, font_dir: &FontDir, font_ident: &Option<Option<&str>>, scaling: Scaling) -> Result<Self, DrawFrameOverlayError> {
        let osd_kind = reader.osd_kind();
        let (overlay_resolution, tile_kind, tile_scaling) = Self::best_settings_for_requested_scaling(osd_kind, &scaling)?;

        let tiles = match font_ident {
            Some(font_ident) => font_dir.load_with_fallback(tile_kind, font_ident, reader.max_used_tile_index().unwrap())?,
            None => font_dir.load_variant_with_fallback(tile_kind, &reader.header().font_variant(), reader.max_used_tile_index().unwrap())?,
        };

        let tile_images = match tile_scaling {
            Some(tile_dimensions) => tiles.as_slice().resized_tiles_par_with_progress(tile_dimensions),
            None => tiles.into_iter().map(|tile| tile.image().clone()).collect(),
        };

        if let Scaling::No { target_resolution: Some(target_resolution) } = scaling {
            let overlay_res_scale =
                (
                    (overlay_resolution.width as f64 / target_resolution.dimensions().width as f64) +
                    (overlay_resolution.height as f64 / target_resolution.dimensions().height as f64)
                ) / 2.0;

            if overlay_res_scale < 0.8 {
                log::warn!("without scaling the overlay resolution is much smaller than the target video resolution, consider using scaling for better results");
            }
        }

        Ok(Self { reader, tile_images, overlay_resolution })
    }

    pub fn draw_next_frame(&mut self) -> Result<Option<Image>, DrawFrameOverlayError> {
        match self.reader.read_frame()? {
            Some(frame) => Ok(Some(self.draw_frame_overlay(&frame).unwrap())),
            None => Ok(None),
        }
    }

    fn transparent_frame_overlay(&self) -> Image {
        Image::new(self.overlay_resolution.width, self.overlay_resolution.height)
    }

    fn draw_frame_overlay(&self, osd_file_frame: &OSDFileFrame) -> Result<Image, DrawFrameOverlayError> {
        let (tiles_width, tiles_height) = self.tile_images.first().unwrap().dimensions();
        let mut image = self.transparent_frame_overlay();
        for (screen_x, screen_y, tile_index) in osd_file_frame.enumerate_tile_indices() {
            image.copy_from(&self.tile_images[tile_index as usize], screen_x as u32 * tiles_width, screen_y as u32 * tiles_height).unwrap();
        }
        Ok(image)
    }

    pub fn save_frames_to_dir<P: AsRef<Path> + std::marker::Sync>(&mut self, path: P, frame_shift: i32) -> Result<(), SaveFramesToDirError> {
        if path.as_ref().exists() {
            return Err(SaveFramesToDirError::TargetDirectoryExists);
        }
        create_path(&path)?;
        log::info!("generating overlay frames and saving into directory: {}", path.as_ref().to_string_lossy());
        let frames = self.reader.frames()?;

        let first_frame_index = frames.iter().position(|frame| (*frame.index() as i32) > -frame_shift).unwrap();
        let frames = &frames[first_frame_index..];
        let first_frame_index = frames.first().unwrap().index();

        let missing_frames = frame_shift + *first_frame_index as i32;

        // we are missing frames at the beginning
        if missing_frames > 0 {
            log::debug!("Generating blank frames 0..{}", missing_frames - 1);
            let frame_0_path = make_overlay_frame_file_path(&path, 0);
            self.transparent_frame_overlay().write_image_file(&frame_0_path)?;
            for frame_index in 1..missing_frames {
                file::hard_link(&frame_0_path, make_overlay_frame_file_path(&path, frame_index as FrameIndex))?;
            }
        }

        let frame_count = *frames.last().unwrap().index();
        let progress_style = ProgressStyle::with_template("{wide_bar} {pos:>6}/{len}").unwrap();
        frames.par_iter().progress_with_style(progress_style).try_for_each(|frame| {
            let actual_frame_index = (*frame.index() as i32 + frame_shift) as u32;
            log::debug!("{} -> {}", frame.index(), &actual_frame_index);
            let frame_image = self.draw_frame_overlay(frame).unwrap();
            frame_image.write_image_file(make_overlay_frame_file_path(&path, actual_frame_index))
        })?;

        log::info!("linking missing overlay frames");
        let frame_indices = frames.iter().map(|x| (*x.index() as i32 + frame_shift) as u32).collect();
        link_missing_frames(&path, &frame_indices)?;

        log::info!("overlay frames generation completed: {} frames", frame_count);
        Ok(())
    }

}