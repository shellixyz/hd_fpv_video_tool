
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

use derive_more::{From, Deref};
use getset::CopyGetters;
use thiserror::Error;
use image::{ImageBuffer, Rgba, GenericImage, ImageResult};
use indicatif::{ProgressStyle, ParallelProgressIterator, ProgressIterator};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator, ParallelBridge, IntoParallelIterator, IndexedParallelIterator};

pub mod scaling;
pub mod margins;
pub mod osd_kind_ext;

use hd_fpv_osd_font_tool::{
    dimensions::Dimensions as GenericDimensions,
    prelude::*,
};

use crate::{
    create_path::{
        CreatePathError,
        create_path,
    },
    file::{
        self,
        HardLinkError,
    },
    image::{
        WriteImageFile,
        WriteError as ImageWriteError,
    },
    video::{
        FrameIndex as VideoFrameIndex,
        resolution::Resolution as VideoResolution, timestamp::{Timestamp, StartEndOverlayFrameIndex},
    },
};

use super::{
    dji::{
        Kind as DJIOSDKind,
        VideoResolutionTooSmallError,
        font_dir::FontDir,
        file::{
            ReadError,
            frame::{
                Frame as OSDFileFrame,
            },
            sorted_frames::{SortedFrames as OSDFileSortedFrames, VideoFramesIter},
        },
    },
    tile_resize::ResizeTiles,
};

use self::scaling::Scaling;

pub type Dimensions = GenericDimensions<u32>;
#[derive(Deref, Clone, CopyGetters)]
pub struct Frame {
    #[getset(get_copy = "pub")]
    dimensions: Dimensions,

    #[deref]
    image: ImageBuffer<Rgba<u8>, Vec<u8>>
}

impl Frame {
    pub fn new(dimensions: Dimensions) -> Self {
        Self { dimensions, image: ImageBuffer::new(dimensions.width, dimensions.height) }
    }

    pub fn copy_from(&mut self, image: &ImageBuffer<Rgba<u8>, Vec<u8>>, x: u32, y: u32) -> ImageResult<()> {
        self.image.copy_from(image, x, y)
    }
}


impl OSDFileFrame {

    fn draw_overlay_frame(&self, dimensions: Dimensions, tile_images: &[tile::Image]) -> Frame {
        let (tiles_width, tiles_height) = tile_images.first().unwrap().dimensions();
        let mut frame = Frame::new(dimensions);
        for (screen_x, screen_y, tile_index) in self.enumerate_tile_indices() {
            frame.copy_from(&tile_images[tile_index as usize], screen_x as u32 * tiles_width, screen_y as u32 * tiles_height).unwrap();
        }
        frame
    }

}


#[derive(Debug, Error, From)]
pub enum DrawFrameOverlayError {
    #[error("OSD file is empty")]
    OSDFileIsEmpty,
    #[error(transparent)]
    ReadError(ReadError),
    #[error("failed to load font file: {0}")]
    FontLoadError(bin_file::LoadError),
    #[error("video resolution {video_resolution} too small to render {osd_kind} OSD kind without scaling")]
    VideoResolutionTooSmallError{ osd_kind: DJIOSDKind, video_resolution: VideoResolution },
}

pub fn format_overlay_frame_file_index(frame_index: VideoFrameIndex) -> String {
    format!("{:010}.png", frame_index)
}

pub fn make_overlay_frame_file_path<P: AsRef<Path>>(dir_path: P, frame_index: VideoFrameIndex) -> PathBuf {
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
    #[error("no frame to write")]
    NoFrameToWrite,
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

fn best_settings_for_requested_scaling(osd_kind: DJIOSDKind, scaling: &Scaling) -> Result<(Dimensions, tile::Kind, Option<TileDimensions>), DrawFrameOverlayError> {
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
                match best_settings_for_requested_scaling(osd_kind, &Scaling::No { target_resolution: Some(target_resolution) }) {

                    // no scaling is possible
                    Ok(values) => {
                        let (overlay_dimensions, _, _) = values;
                        let (margin_width, margin_height) = crate::video::utils::margins(target_resolution.dimensions(), overlay_dimensions);
                        let min_margins_condition_met = margin_width >= min_margins.horizontal() as i32 && margin_height >= min_margins.vertical() as i32;
                        let min_dimensions_condition_met = overlay_dimensions.width >= min_resolution.width && overlay_dimensions.height >= min_resolution.height;

                        // check whether the result would match the user specified conditions
                        if min_margins_condition_met && min_dimensions_condition_met {
                            values
                        } else {
                            // else return parameters with scaling enabled
                            best_settings_for_requested_scaling(osd_kind, &Scaling::Yes { target_resolution, min_margins })?
                        }

                    },

                    // no scaling does not work, return parameters with scaling enabled
                    Err(_) => best_settings_for_requested_scaling(osd_kind, &Scaling::Yes { target_resolution, min_margins })?,
                };

            let tile_scaling_yes_no = match tile_scaling { Some(_) => "yes", None => "no" };
            log::info!("calculated best approach: tile kind: {tile_kind} - scaling {tile_scaling_yes_no} - overlay resolution {overlay_resolution}");

            (overlay_resolution, tile_kind, tile_scaling)
        },
    })
}

#[derive(CopyGetters)]
pub struct Generator {
    osd_file_frames: OSDFileSortedFrames,
    tile_images: Vec<tile::Image>,

    #[getset(get_copy = "pub")]
    frame_dimensions: Dimensions,
}

impl Generator {

    pub fn new(osd_file_frames: OSDFileSortedFrames, font_dir: &FontDir, font_ident: &Option<Option<&str>>, scaling: Scaling) -> Result<Self, DrawFrameOverlayError> {
        if osd_file_frames.is_empty() { return Err(DrawFrameOverlayError::OSDFileIsEmpty) }

        let (overlay_resolution, tile_kind, tile_scaling) = best_settings_for_requested_scaling(osd_file_frames.kind(), &scaling)?;

        let highest_used_tile_index = osd_file_frames.highest_used_tile_index().unwrap();
        let tiles = match font_ident {
            Some(font_ident) => font_dir.load_with_fallback(tile_kind, font_ident, highest_used_tile_index)?,
            None => font_dir.load_variant_with_fallback(tile_kind, &osd_file_frames.font_variant(), highest_used_tile_index)?,
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

        Ok(Self { osd_file_frames, tile_images, frame_dimensions: overlay_resolution })
    }

    fn draw_frame(&self, osd_file_frame: &OSDFileFrame) -> Frame {
        osd_file_frame.draw_overlay_frame(self.frame_dimensions, &self.tile_images)
    }

    fn link_missing_frames<P: AsRef<Path>>(dir_path: P, existing_frame_indices: &BTreeSet<VideoFrameIndex>) -> Result<(), IOError> {
        if ! existing_frame_indices.is_empty() {
            let existing_frame_indices_vec = existing_frame_indices.iter().collect::<Vec<&VideoFrameIndex>>();
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
        }
        Ok(())
    }

    pub fn save_frames_to_dir<P: AsRef<Path> + std::marker::Sync>(&mut self, start: Option<Timestamp>, end: Option<Timestamp>, path: P, frame_shift: i32) -> Result<(), SaveFramesToDirError> {
        if path.as_ref().exists() {
            return Err(SaveFramesToDirError::TargetDirectoryExists);
        }
        create_path(&path)?;
        log::info!("generating overlay frames and saving into directory: {}", path.as_ref().to_string_lossy());

        // let first_video_frame_index = start.start_overlay_frame_count() as i32 - frame_shift;
        // let first_frame_index = self.osd_file_frames.iter().position(|frame| (frame.index() as i32) >= first_video_frame_index);
        // // let first_frame_index = self.osd_file_frames.iter().position(|frame| (frame.index() as i32) > -frame_shift).unwrap();
        // // let frames = &self.osd_file_frames[first_frame_index..];
        // let frames = first_frame_index.map(|index| &self.osd_file_frames[index..]).unwrap_or(&[]);
        // let first_frame_index = frames.first().unwrap().index();

        // let missing_frames = frame_shift + first_frame_index as i32;

        let first_video_frame = start.start_overlay_frame_count();
        let last_video_frame = end.end_overlay_frame_index();

        let frames_iter = self.osd_file_frames.par_shift_iter(first_video_frame, last_video_frame, frame_shift);
        let frame_count = frames_iter.len();
        if frame_count == 0 { return Err(SaveFramesToDirError::NoFrameToWrite); }

        let first_frame_index = self.osd_file_frames.first_video_frame_index(first_video_frame, frame_shift).unwrap();

        // we are missing frames at the beginning
        if first_frame_index > 0 {
            log::debug!("Generating blank frames 0..{}", first_frame_index - 1);
            let frame_0_path = make_overlay_frame_file_path(&path, 0);
            Frame::new(self.frame_dimensions).write_image_file(&frame_0_path)?;
            for frame_index in 1..first_frame_index {
                file::hard_link(&frame_0_path, make_overlay_frame_file_path(&path, frame_index as VideoFrameIndex))?;
            }
        }

        // let frame_count = frames.last().unwrap().index();
        let progress_style = ProgressStyle::with_template("{wide_bar} {pos:>6}/{len}").unwrap();
        frames_iter.progress_with_style(progress_style).try_for_each(|(frame_index, frame)| {
            let dir_frame_index = (frame_index as i32 - first_video_frame as i32) as u32;
            log::debug!("{} -> {}", frame.index(), &dir_frame_index);
            let frame_image = self.draw_frame(frame);
            frame_image.write_image_file(make_overlay_frame_file_path(&path, dir_frame_index))
        })?;

        log::info!("linking missing overlay frames");
        // let frame_indices = frames.iter().map(|frame| (frame.index() as i32 + frame_shift) as u32).collect();
        let frame_indices = self.osd_file_frames.video_frame_indices(first_video_frame, last_video_frame, frame_shift);
        Self::link_missing_frames(&path, &frame_indices)?;

        log::info!("overlay frames generation completed: {} frames", frame_count);
        Ok(())
    }

    pub fn generate_overlay_video<P: AsRef<Path>>(&mut self, start: Option<Timestamp>, end: Option<Timestamp>, output_video_path: P, frame_shift: i32) -> Result<(), GenerateOverlayVideoError> {
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
                "-video_size", self.frame_dimensions.to_string().as_str(),
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

        let progress_style = ProgressStyle::with_template("{wide_bar} {percent:>3}% [ETA {eta:>3}]").unwrap();
        // let mut prev_frame_image = Frame::new(self.frame_dimensions);
        // let mut video_frames_iter = self.osd_file_frames.video_frames_iter(0, None, frame_shift);
        // for osd_file_frame in video_frames_iter.progress_with_style(progress_style) {
        //     match osd_file_frame {
        //         Some(osd_file_frame) => {
        //             let new_frame_image = self.draw_frame(osd_file_frame);
        //             ffmpeg_stdin.write_all(new_frame_image.as_raw())?;
        //             prev_frame_image = new_frame_image;
        //         },
        //         None => ffmpeg_stdin.write_all(prev_frame_image.as_raw())?,
        //     }
        // }
        let frames_iter = self.iter_advanced(start.start_overlay_frame_count(), end.end_overlay_frame_index(), frame_shift);
        let frame_count = frames_iter.len();
        for frame in frames_iter.progress_with_style(progress_style) {
            ffmpeg_stdin.write_all(frame.as_raw())?;
        }

        drop(ffmpeg_stdin);

        let ffmpeg_result = ffmpeg_child.wait()?;
        if !ffmpeg_result.success() {
            return Err(GenerateOverlayVideoError::FFMpegExitedWithError(ffmpeg_result.code().unwrap()))
        }

        log::info!("overlay video generation completed: {} frames", frame_count);
        Ok(())
    }

    pub fn iter(&self) -> FramesIter {
        self.into_iter()
    }

    pub fn iter_advanced(&self, first_frame: u32, last_frame: Option<u32>, frame_shift: i32) -> FramesIter {
        self.osd_file_frames.overlay_frames_iter(self.frame_dimensions, first_frame, last_frame, frame_shift, &self.tile_images)
    }

}

impl<'a> IntoIterator for &'a Generator {
    type Item = Frame;

    type IntoIter = FramesIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.osd_file_frames.overlay_frames_iter(self.frame_dimensions, 0, None, 0, &self.tile_images)
    }
}

impl OSDFileSortedFrames {
    pub fn overlay_frames_iter<'a>(&'a self, frame_dimensions: Dimensions, first_frame: u32, last_frame: Option<u32>, frame_shift: i32, tile_images: &'a [tile::Image]) -> FramesIter {
        FramesIter::new(self.video_frames_iter(first_frame, last_frame, frame_shift), frame_dimensions, tile_images)
    }
}

#[derive(CopyGetters)]
pub struct FramesIter<'a> {
    #[getset(get_copy = "pub")]
    frame_dimensions: Dimensions,
    tile_images: &'a [tile::Image],
    vframes_iter: VideoFramesIter<'a>,
    prev_frame: Frame
}

impl<'a> FramesIter<'a> {
    pub fn new(video_frames_iter: VideoFramesIter<'a>, frame_dimensions: Dimensions, tile_images: &'a [tile::Image]) -> Self {
        Self {
            frame_dimensions,
            tile_images,
            vframes_iter: video_frames_iter,
            prev_frame: Frame::new(frame_dimensions)
        }
    }
}

impl<'a> Iterator for FramesIter<'a> {
    type Item = Frame;

    fn next(&mut self) -> Option<Self::Item> {
        match self.vframes_iter.next()? {
            Some(osd_file_frame) => {
                let frame = osd_file_frame.draw_overlay_frame(self.frame_dimensions, self.tile_images);
                self.prev_frame = frame.clone();
                Some(frame)
            },
            None => Some(self.prev_frame.clone()),
        }
    }
}

impl<'a> ExactSizeIterator for FramesIter<'a> {
    fn len(&self) -> usize {
        self.vframes_iter.len()
    }
}