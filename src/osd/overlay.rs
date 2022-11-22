
use std::{
    collections::BTreeSet,
    path::{
        Path,
        PathBuf
    },
    io::{
        Error as IOError,
        Write
    },
    process::{
        Command,
        Stdio
    },
};

use derive_more::From;
use getset::CopyGetters;
use thiserror::Error;
use image::{ImageBuffer, Rgba, GenericImage};
use indicatif::{ProgressStyle, ParallelProgressIterator, ProgressIterator};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

pub mod resolution;
pub mod scaling;
pub mod margins;

use hd_fpv_osd_font_tool::prelude::*;

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
    }, cli::{font_options::FontOptions, osd_args::OSDArgs},
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

use self::{
    resolution::{
        Resolution,
        VideoResolution,
    },
    scaling::{Scaling, ScalingArgs},
};


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

#[derive(Debug, Error, From)]
pub enum GenerateOverlayVideoError {
    #[error(transparent)]
    FrameReadError(ReadError),
    #[error("target video file exists")]
    TargetVideoFileExists,
    #[error("failed spawning ffmpeg process: {0}")]
    #[from(ignore)]
    FailedSpawningFFMpegProcess(IOError),
    #[error("failed talking to ffmpeg process: {0}")]
    FailedTalkingToFFMpegProcess(IOError),
    #[error("ffmpeg process exited with error: {0}")]
    FFMpegExitedWithError(i32),
}

#[derive(CopyGetters)]
pub struct Generator {
    reader: OSDFileReader,
    tile_images: Vec<tile::Image>,

    #[getset(get_copy = "pub")]
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
                    target_resolution.dimensions().width - 2 * min_margins.horizontal(),
                    target_resolution.dimensions().height - 2 * min_margins.vertical(),
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
                            let min_margins_condition_met = margin_width >= min_margins.horizontal() as i32 && margin_height >= min_margins.vertical() as i32;
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

    pub fn new_from_cli_args(scaling_args: &ScalingArgs, font_options: &FontOptions, osd_args: &OSDArgs) -> anyhow::Result<Self> {
        let scaling = Scaling::try_from(scaling_args)?;
        let osd_file = OSDFileReader::open(osd_args.osd_file())?;
        let font_dir = FontDir::new(&font_options.font_dir());
        let overlay_generator = osd_file.into_frame_overlay_generator(
            &font_dir,
            &font_options.font_ident(),
            scaling
        )?;
        Ok(overlay_generator)
    }

    pub fn draw_next_frame(&mut self) -> Result<Option<Image>, DrawFrameOverlayError> {
        match self.reader.read_frame()? {
            Some(frame) => Ok(Some(self.draw_frame_overlay(&frame))),
            None => Ok(None),
        }
    }

    fn transparent_frame_overlay(&self) -> Image {
        Image::new(self.overlay_resolution.width, self.overlay_resolution.height)
    }

    fn draw_frame_overlay(&self, osd_file_frame: &OSDFileFrame) -> Image {
        let (tiles_width, tiles_height) = self.tile_images.first().unwrap().dimensions();
        let mut image = self.transparent_frame_overlay();
        for (screen_x, screen_y, tile_index) in osd_file_frame.enumerate_tile_indices() {
            image.copy_from(&self.tile_images[tile_index as usize], screen_x as u32 * tiles_width, screen_y as u32 * tiles_height).unwrap();
        }
        image
    }

    fn link_missing_frames<P: AsRef<Path>>(dir_path: P, existing_frame_indices: &BTreeSet<FrameIndex>) -> Result<(), IOError> {
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
            let frame_image = self.draw_frame_overlay(frame);
            frame_image.write_image_file(make_overlay_frame_file_path(&path, actual_frame_index))
        })?;

        log::info!("linking missing overlay frames");
        let frame_indices = frames.iter().map(|x| (*x.index() as i32 + frame_shift) as u32).collect();
        Self::link_missing_frames(&path, &frame_indices)?;

        log::info!("overlay frames generation completed: {} frames", frame_count);
        Ok(())
    }

    pub fn generate_overlay_video<P: AsRef<Path>>(&mut self, output_video_path: P, frame_shift: i32) -> Result<(), GenerateOverlayVideoError> {
        if output_video_path.as_ref().exists() {
            return Err(GenerateOverlayVideoError::TargetVideoFileExists);
        }
        log::info!("generating overlay video: {}", output_video_path.as_ref().to_string_lossy());

        let mut ffmpeg_command = Command::new("ffmpeg");
        let ffmpeg_command_with_args =
            ffmpeg_command
            .args([
                "-f", "rawvideo",
                "-pix_fmt", "rgba",
                "-video_size", self.overlay_resolution.to_string().as_str(),
                "-r", "60",
                "-i", "pipe:0",
                "-c:v", "libvpx-vp9",
                "-crf", "40",
                "-b:v", "0",
                "-y",
                "-pix_fmt", "yuva420p",
            ])
            .arg(output_video_path.as_ref().as_os_str());

        let mut ffmpeg_child = ffmpeg_command_with_args
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(GenerateOverlayVideoError::FailedSpawningFFMpegProcess)?;
        let mut ffmpeg_stdin = ffmpeg_child.stdin.take().expect("failed to open ffmpeg stdin");

        let transparent_frame_image = self.transparent_frame_overlay();

        let frame_count = self.reader.last_frame_frame_index()?;
        let progress_style = ProgressStyle::with_template("{wide_bar} {percent:>3}% [ETA {eta:>3}]").unwrap();
        let mut prev_frame_image = transparent_frame_image;
        let mut current_frame = self.reader.read_frame()?.unwrap();
        for frame_index in (0..frame_count).progress_with_style(progress_style) {
            let actual_frame_index = (frame_index as i32 + frame_shift) as u32;
            log::debug!("{} -> {}", &frame_index, &actual_frame_index);
            debug_assert!(frame_index <= *current_frame.index());
            if frame_index == *current_frame.index() {
                let frame_image = self.draw_frame_overlay(&current_frame);
                ffmpeg_stdin.write_all(frame_image.as_raw())?;
                prev_frame_image = frame_image;
                if frame_index < frame_count - 1 {
                    current_frame = self.reader.read_frame()?.unwrap();
                }
            } else {
                ffmpeg_stdin.write_all(prev_frame_image.as_raw())?;
            }
        };

        drop(ffmpeg_stdin);

        let ffmpeg_result = ffmpeg_child.wait()?;
        if !ffmpeg_result.success() {
            return Err(GenerateOverlayVideoError::FFMpegExitedWithError(ffmpeg_result.code().unwrap()))
        }

        log::info!("overlay video generation completed: {} frames", frame_count);
        Ok(())
    }

    pub fn into_iter(mut self, first_frame: u32, last_frame: Option<u32>, frame_shift: i32) -> Result<FramesIntoIter, ReadError> {

        let frames = self.reader.frames()?;

        let first_frame_index = first_frame as i32 - frame_shift;
        let first_osd_file_frame_index = frames.iter().position(|frame| (*frame.index() as i32) > first_frame_index).unwrap();
        let osd_file_frames = frames[first_osd_file_frame_index..].to_vec();
        let transparent_frame = self.transparent_frame_overlay();

        Ok(FramesIntoIter {
            generator: self,
            osd_file_frames,
            osd_file_frame_index: 0,
            current_frame_index: 0,
            first_frame,
            last_frame,
            frame_shift,
            prev_frame: transparent_frame,
        })
    }

}

pub struct FramesIntoIter {
    generator: Generator,
    osd_file_frames: Vec<OSDFileFrame>,
    osd_file_frame_index: usize,
    current_frame_index: u32,
    first_frame: u32,
    last_frame: Option<u32>,
    frame_shift: i32,
    prev_frame: Image,
}

impl Iterator for FramesIntoIter {
    type Item = Image;

    fn next(&mut self) -> Option<Self::Item> {
        if self.osd_file_frame_index > self.osd_file_frames.len() - 1
            || self.last_frame.map(|last_frame| self.current_frame_index > last_frame).unwrap_or(false) {
            return None;
            // match self.last_frame {
            //     Some(last_frame) => {
            //         if self.current_frame_index > last_frame {
            //             return None;
            //         } else {
            //             self.current_frame_index += 1;
            //             return Some(self.prev_frame.clone());
            //         }
            //     },
            //     None => return None,
            // }
        }

        let osd_file_frame = &self.osd_file_frames[self.osd_file_frame_index];
        let actual_osd_file_frame_frame_index = *osd_file_frame.index() as i32 + self.frame_shift;

        let frame =
            if (self.current_frame_index as i32) < actual_osd_file_frame_frame_index {
                self.prev_frame.clone()
            } else {
                let frame = self.generator.draw_frame_overlay(osd_file_frame);
                self.osd_file_frame_index += 1;
                self.prev_frame = frame.clone();
                frame
            };

        self.current_frame_index += 1;

        Some(frame)
    }
}