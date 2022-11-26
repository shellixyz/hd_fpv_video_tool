
use std::{
    path::{
        Path,
        PathBuf
    },
    io::{
        Error as IOError,
        Write
    },
};

use derive_more::{From, Deref};
use getset::{CopyGetters, Getters};
use path_absolutize::Absolutize;
use thiserror::Error;
use image::{ImageBuffer, Rgba, GenericImage, ImageResult};
use indicatif::{ProgressStyle, ParallelProgressIterator, ProgressBar};
use rayon::prelude::{ParallelIterator, IndexedParallelIterator};

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
    ffmpeg,
    file::{
        self,
        HardLinkError, SymlinkError,
    },
    image::{
        WriteImageFile,
        WriteError as ImageWriteError,
    },
    video::{
        FrameIndex as VideoFrameIndex,
        resolution::Resolution as VideoResolution, timestamp::{Timestamp, StartEndOverlayFrameIndex},
    }, osd::dji::file::sorted_frames::{GetFramesExt, EndOfFramesAction},
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
            sorted_frames::{SortedUniqFrames as OSDFileSortedFrames, VideoFramesIter},
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


#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum OverlayVideoCodec {
    Vp8,
    Vp9
}

#[derive(Debug, Clone, Getters, CopyGetters)]
#[getset(get_copy = "pub")]
pub struct OverlayVideoCodecParams {
    encoder: &'static str,
    bitrate: Option<&'static str>,
    crf: Option<u8>,

    #[getset(skip)]
    #[getset(get = "pub")]
    additional_args: Vec<&'static str>,
}

impl OverlayVideoCodecParams {
    pub fn new(encoder: &'static str, bitrate: Option<&'static str>, crf: Option<u8>, additional_args: &[&'static str]) -> Self {
        Self {
            encoder,
            bitrate,
            crf,
            additional_args: additional_args.to_vec(),
        }
    }
}

impl OverlayVideoCodec {
    pub fn params(&self) -> OverlayVideoCodecParams {
        use OverlayVideoCodec::*;
        match self {
            Vp8 => OverlayVideoCodecParams::new("libvpx", Some("1M"), Some(40), &["-auto-alt-ref", "0"]),
            Vp9 => OverlayVideoCodecParams::new("libvpx-vp9", Some("0"), Some(40), &[]),
        }
    }
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
    #[error(transparent)]
    SymlinkError(SymlinkError),
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
    #[error("output video file extension needs to be .webm")]
    OutputFileExtensionNotWebm,
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

    pub fn save_frames_to_dir<P: AsRef<Path> + std::marker::Sync>(&mut self, start: Option<Timestamp>, end: Option<Timestamp>, path: P, frame_shift: i32) -> Result<(), SaveFramesToDirError> {
        if path.as_ref().exists() {
            return Err(SaveFramesToDirError::TargetDirectoryExists);
        }
        create_path(&path)?;
        log::info!("generating overlay frames and saving into directory: {}", path.as_ref().to_string_lossy());

        let first_video_frame = start.start_overlay_frame_count();
        let last_video_frame = end.end_overlay_frame_index();

        let osd_file_frames_slice = self.osd_file_frames.select_slice(first_video_frame, last_video_frame, frame_shift);
        if osd_file_frames_slice.is_empty() { return Err(SaveFramesToDirError::NoFrameToWrite); }

        let iter = osd_file_frames_slice.video_frames_rel_index_par_iter(EndOfFramesAction::ContinueToLastVideoFrame);
        let frame_count = iter.len();

        let progress_style = ProgressStyle::with_template("{wide_bar} {pos:>6}/{len}").unwrap();
        let progress_bar = ProgressBar::new(frame_count as u64).with_style(progress_style);
        progress_bar.enable_steady_tick(std::time::Duration::new(0, 100_000_000));

        iter.progress_with(progress_bar).try_for_each(|item| {
            use crate::osd::dji::file::sorted_frames::VideoFramesRelIndexIterItem::*;
            match item {
                Existing { rel_index, frame } => {
                    log::debug!("existing {}", &rel_index);
                    let frame_image = self.draw_frame(frame);
                    frame_image.write_image_file(make_overlay_frame_file_path(&path, rel_index))?;
                },
                FirstNonExisting => {
                    log::debug!("first non existing");
                    let frame_0_path = make_overlay_frame_file_path(&path, 0);
                    Frame::new(self.frame_dimensions).write_image_file(&frame_0_path)?;
                },
                NonExisting { prev_rel_index, rel_index } => {
                    log::debug!("non existing {} -> {}", rel_index, prev_rel_index);
                    let prev_path = make_overlay_frame_file_path(&path, prev_rel_index);
                    let link_path = make_overlay_frame_file_path(&path, rel_index);
                    let abs_link_path = link_path.absolutize().unwrap();
                    file::symlink(prev_path, abs_link_path)?;
                },
            }
            Ok::<(), SaveFramesToDirError>(())
        })?;

        log::info!("overlay frames generation completed: {} frames files written", frame_count);
        Ok(())
    }

    pub async fn generate_overlay_video<P: AsRef<Path>>(&mut self, codec: OverlayVideoCodec, start: Option<Timestamp>, end: Option<Timestamp>, output_video_path: P, frame_shift: i32, overwrite_output: bool) -> Result<(), GenerateOverlayVideoError> {

        if ! matches!(output_video_path.as_ref().extension(), Some(extension) if extension == "webm") { return Err(GenerateOverlayVideoError::OutputFileExtensionNotWebm) }
        if ! overwrite_output &&  output_video_path.as_ref().exists() { return Err(GenerateOverlayVideoError::TargetVideoFileExists); }

        log::info!("generating overlay video: {}", output_video_path.as_ref().to_string_lossy());

        let frames_iter = self.iter_advanced(start.start_overlay_frame_count(), end.end_overlay_frame_index(), frame_shift);
        let frame_count = frames_iter.len();

        let mut ffmpeg_command = ffmpeg::CommandBuilder::default();

        ffmpeg_command
            .add_stdin_input(self.frame_dimensions, 60).unwrap()
            .set_output_video_settings(Some(codec.params().encoder()), codec.params().bitrate(), codec.params().crf())
            .add_args(codec.params().additional_args())
            .set_output_file(output_video_path)
            .set_overwrite_output_file(overwrite_output);

        let mut ffmpeg_process = ffmpeg_command.build().unwrap().spawn_with_progress(frame_count as u64).unwrap();
        let mut ffmpeg_stdin = ffmpeg_process.take_stdin().unwrap();

        for osd_frame_image in frames_iter {
            ffmpeg_stdin.write_all(osd_frame_image.as_raw())?;
        }

        drop(ffmpeg_stdin);

        if let Err(error) = ffmpeg_process.wait().await {
            return Err(GenerateOverlayVideoError::FFMpegExitedWithError(error.exit_status().code().unwrap()))
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