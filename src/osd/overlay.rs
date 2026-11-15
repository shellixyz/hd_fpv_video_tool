use std::{
	io::{self, Error as IOError, Write},
	path::{Path, PathBuf},
};

use derive_more::{Deref, From};
use getset::{CopyGetters, Getters};
use image::{GenericImage, ImageBuffer, ImageResult, Rgba};
use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};
use itertools::Itertools;
use path_absolutize::Absolutize;
use rayon::prelude::{IndexedParallelIterator, ParallelIterator};
use thiserror::Error;
use tokio::io::AsyncWriteExt as _;

pub mod margins;
pub mod osd_kind_ext;
pub mod scaling;

use hd_fpv_osd_font_tool::{dimensions::Dimensions as GenericDimensions, prelude::*};

use self::scaling::Scaling;
use super::{
	FontDir, Region,
	file::{
		Frame as OSDFileFrame, ReadError, SortedUniqFrames as OSDFileSortedFrames,
		sorted_frames::{GetFrames, GetFramesExt, VideoFramesIter},
	},
	font_variant::FontVariant,
	tile_indices::UnknownOSDItem,
	tile_resize::ResizeTiles,
};
use crate::{
	create_path::{CreatePathError, create_path},
	ffmpeg::{self, VideoQuality},
	file::{self, TouchError},
	image::{WriteError as ImageWriteError, WriteImageFile},
	osd::file::sorted_frames::EndOfFramesAction,
	video::{
		Codec, FrameIndex as VideoFrameIndex, HwAcceleratedEncoding,
		resolution::Resolution as VideoResolution,
		timestamp::{StartEndOverlayFrameIndex, Timestamp},
	},
};

pub type Dimensions = GenericDimensions<u32>;

/// Alignment of the alpha plane's left edge in a packed-alpha frame, so that encoder blocks do not
/// straddle the color and alpha planes.
const PACKED_ALPHA_PLANE_ALIGNMENT: u32 = 64;

/// Placement of an overlay's color and alpha planes inside a packed-alpha video frame.
///
/// The color plane holds the overlay's straight RGB at the frame origin. The alpha plane holds its
/// alpha as gray, at the same height, starting at [`Self::alpha_x`]. Gray converts to luma
/// independently of the YUV matrix, so codecs without an alpha channel carry it intact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, CopyGetters)]
#[getset(get_copy = "pub")]
pub struct PackedAlphaLayout {
	overlay_dimensions: Dimensions,
	alpha_x: u32,
}

impl PackedAlphaLayout {
	#[must_use]
	pub fn new(overlay_dimensions: Dimensions) -> Self {
		Self {
			overlay_dimensions,
			alpha_x: overlay_dimensions.width.next_multiple_of(PACKED_ALPHA_PLANE_ALIGNMENT),
		}
	}

	/// Reads a layout from sidecar metadata values: the overlay size as `<width>x<height>` and the
	/// alpha plane's left edge.
	#[must_use]
	pub fn from_metadata(size: &str, alpha_x: &str) -> Option<Self> {
		let overlay_dimensions: Dimensions = size.parse().ok()?;
		let alpha_x = alpha_x.parse().ok()?;
		(overlay_dimensions.width > 0 && overlay_dimensions.height > 0 && alpha_x >= overlay_dimensions.width)
			.then_some(Self {
				overlay_dimensions,
				alpha_x,
			})
	}

	/// Dimensions of the encoded frame holding both planes.
	#[must_use]
	pub fn packed_dimensions(self) -> Dimensions {
		Dimensions::new(
			self.alpha_x + self.overlay_dimensions.width,
			self.overlay_dimensions.height,
		)
	}

	/// Builds an `FFmpeg` filter graph fragment that turns the packed frames from the `input` label
	/// into RGBA overlay frames on the `output` label.
	///
	/// Both planes are cropped at explicit coordinates, so padding added by the encoder to the
	/// right or bottom of the coded frame is discarded.
	#[must_use]
	pub fn unpack_filter(self, input: &str, output: &str) -> String {
		let Dimensions { width, height } = self.overlay_dimensions;
		let alpha_x = self.alpha_x;
		format!(
			"[{input}]split[osd_packed_color][osd_packed_alpha];[osd_packed_color]crop={width}:{height}:0:\
			 0[osd_color];[osd_packed_alpha]crop={width}:{height}:{alpha_x}:0,format=gray[osd_alpha];\
			 [osd_color][osd_alpha]alphamerge[{output}]"
		)
	}
}

#[derive(Deref, Clone, CopyGetters)]
pub struct Frame {
	#[getset(get_copy = "pub")]
	dimensions: Dimensions,

	#[deref]
	image: ImageBuffer<Rgba<u8>, Vec<u8>>,
}

#[derive(Debug, Error)]
#[error("video resolution {video_resolution} too small to fit {osd_kind} kind OSD")]
pub struct VideoResolutionTooSmallError {
	pub osd_kind: super::Kind,
	pub video_resolution: VideoResolution,
}

impl Frame {
	#[must_use]
	pub fn new(dimensions: Dimensions) -> Self {
		Self {
			dimensions,
			image: ImageBuffer::new(dimensions.width, dimensions.height),
		}
	}

	/// Copies pixels from the provided image into this frame at the specified (x, y) position
	///
	/// # Errors
	/// See `image::GenericImage::copy_from` for details
	pub fn copy_from(&mut self, image: &ImageBuffer<Rgba<u8>, Vec<u8>>, x: u32, y: u32) -> ImageResult<()> {
		self.image.copy_from(image, x, y)
	}

	/// Packs this frame side by side into an opaque color plane and a gray alpha plane.
	///
	/// # Panics
	/// Panics if `layout` was made for different overlay dimensions.
	#[must_use]
	pub fn pack_alpha(&self, layout: PackedAlphaLayout) -> Self {
		assert_eq!(
			layout.overlay_dimensions(),
			self.dimensions,
			"packed-alpha layout does not match the frame"
		);
		let mut packed = Self::new(layout.packed_dimensions());
		for (x, y, pixel) in self.image.enumerate_pixels() {
			let [red, green, blue, alpha] = pixel.0;
			packed.image.put_pixel(x, y, Rgba([red, green, blue, u8::MAX]));
			packed
				.image
				.put_pixel(layout.alpha_x + x, y, Rgba([alpha, alpha, alpha, u8::MAX]));
		}
		packed
	}
}

impl super::file::Frame {
	fn draw_overlay_frame(
		&self,
		dimensions: Dimensions,
		font_variant: FontVariant,
		tile_images: &[tile::Image],
		hidden_regions: &[Region],
		hidden_items: &[impl AsRef<str>],
	) -> Result<Frame, UnknownOSDItem> {
		let (tiles_width, tiles_height) = tile_images.first().unwrap().dimensions();
		let mut frame = Frame::new(dimensions);
		let mut tile_indices = self.tile_indices().clone();
		tile_indices.erase_regions(hidden_regions);
		tile_indices.erase_osd_items(font_variant, hidden_items)?;
		for (osd_coordinates, tile_index) in tile_indices.enumerate() {
			let Some(tile_image) = tile_images.get(tile_index as usize) else {
				continue;
			};
			let x = u32::from(osd_coordinates.x) * tiles_width;
			let y = u32::from(osd_coordinates.y) * tiles_height;
			if x < frame.width() && y < frame.height() {
				frame
					.copy_from(
						tile_image,
						u32::from(osd_coordinates.x) * tiles_width,
						u32::from(osd_coordinates.y) * tiles_height,
					)
					.unwrap();
			}
		}
		Ok(frame)
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
	VideoResolutionTooSmallError {
		osd_kind: super::Kind,
		video_resolution: VideoResolution,
	},
}

#[must_use]
pub fn format_overlay_frame_file_index(frame_index: VideoFrameIndex) -> String {
	format!("{frame_index:010}.png")
}

/// Constructs the file path for an overlay frame image file given a directory path and a frame
/// index.
///
/// # Panics
/// Panics if the directory path cannot be converted to a string slice (i.e., if it contains invalid
/// UTF-8).
pub fn make_overlay_frame_file_path<P: AsRef<Path>>(dir_path: P, frame_index: VideoFrameIndex) -> PathBuf {
	[
		dir_path.as_ref().to_str().unwrap(),
		&format_overlay_frame_file_index(frame_index),
	]
	.iter()
	.collect()
}

#[derive(Debug, Default, Clone, Copy, clap::ValueEnum)]
pub enum OverlayVideoCodec {
	/// MP4 with the overlay's color and alpha packed side by side, using the default video encoder
	#[default]
	PackedAlpha,
	/// transparent VP8 `WebM`
	Vp8,
	/// transparent VP9 `WebM`
	Vp9,
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
	#[must_use]
	pub fn new(
		encoder: &'static str,
		bitrate: Option<&'static str>,
		crf: Option<u8>,
		additional_args: &[&'static str],
	) -> Self {
		Self {
			encoder,
			bitrate,
			crf,
			additional_args: additional_args.to_vec(),
		}
	}
}

impl OverlayVideoCodec {
	#[must_use]
	pub fn file_extension(self) -> &'static str {
		match self {
			Self::PackedAlpha => "mp4",
			Self::Vp8 | Self::Vp9 => "webm",
		}
	}

	#[must_use]
	pub fn is_packed_alpha(self) -> bool {
		matches!(self, Self::PackedAlpha)
	}

	/// Returns the packed-alpha layout of this codec's frames for the given overlay dimensions.
	#[must_use]
	pub fn packed_alpha_layout(self, overlay_dimensions: Dimensions) -> Option<PackedAlphaLayout> {
		self.is_packed_alpha()
			.then(|| PackedAlphaLayout::new(overlay_dimensions))
	}

	#[must_use]
	pub fn params(&self) -> Option<OverlayVideoCodecParams> {
		use OverlayVideoCodec as OVC;
		use OverlayVideoCodecParams as OVCP;
		Some(match self {
			OVC::PackedAlpha => return None,
			OVC::Vp8 => OVCP::new("libvpx", Some("1M"), Some(40), &["-auto-alt-ref", "0"]),
			OVC::Vp9 => OVCP::new("libvpx-vp9", Some("0"), Some(40), &[]),
		})
	}
}

pub const OVERLAY_MODE_METADATA: &str = "hd_fpv_overlay_mode";
pub const PACKED_ALPHA_MODE: &str = "packed-alpha";
pub const OVERLAY_SIZE_METADATA: &str = "hd_fpv_overlay_size";
pub const OVERLAY_ALPHA_X_METADATA: &str = "hd_fpv_overlay_alpha_x";
const PACKED_ALPHA_BITRATE_CAP: &str = "4M";

/// Constant-quality setting for packed-alpha sidecars. VAAPI's `global_quality` is a quantizer
/// whose scale depends on the codec: 0-255 for AV1, 0-51 for H.264 and H.265.
fn packed_alpha_quality(codec: Codec, hw_acceleration: HwAcceleratedEncoding) -> VideoQuality {
	match (hw_acceleration, codec) {
		(HwAcceleratedEncoding::Yes, Codec::AV1) => VideoQuality::GlobalQuality(90),
		(HwAcceleratedEncoding::Yes, _) => VideoQuality::GlobalQuality(24),
		(HwAcceleratedEncoding::No, _) => VideoQuality::ConstantRateFactor(28),
	}
}

fn configure_overlay_video_output(
	ffmpeg_command: &mut ffmpeg::CommandBuilder,
	codec: OverlayVideoCodec,
	packed_alpha_layout: Option<PackedAlphaLayout>,
) {
	if let Some(layout) = packed_alpha_layout {
		let (video_codec, hw_acceleration) = crate::video::default_video_codec(false);
		let quality = packed_alpha_quality(video_codec, hw_acceleration);
		ffmpeg_command
			.set_output_video_settings(
				Some(video_codec.ffmpeg_string(hw_acceleration.is_yes())),
				Some(PACKED_ALPHA_BITRATE_CAP),
				Some(quality),
			)
			.add_arg("-an")
			.add_arg("-movflags")
			.add_arg("use_metadata_tags")
			.add_arg("-metadata")
			.add_arg(&format!("{OVERLAY_MODE_METADATA}={PACKED_ALPHA_MODE}"))
			.add_arg("-metadata")
			.add_arg(&format!("{OVERLAY_SIZE_METADATA}={}", layout.overlay_dimensions()))
			.add_arg("-metadata")
			.add_arg(&format!("{OVERLAY_ALPHA_X_METADATA}={}", layout.alpha_x()));
		if hw_acceleration.is_yes() {
			ffmpeg_command.add_video_filter("format=nv12,hwupload");
			#[cfg(feature = "hwaccel")]
			if let Some(device) = crate::video::hw_accel::vaapi_device_path() {
				ffmpeg_command
					.add_prefix_arg("-vaapi_device")
					.add_prefix_arg(device.to_str().unwrap_or("/dev/dri/renderD128"));
			}
		}
	} else if let Some(params) = codec.params() {
		ffmpeg_command
			.set_output_video_settings(
				Some(params.encoder()),
				params.bitrate(),
				params.crf().map(VideoQuality::ConstantRateFactor),
			)
			.add_args(params.additional_args());
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
	#[from(ignore)]
	SymlinkError(IOError),
	#[error("no frame to write")]
	NoFrameToWrite,
	#[error("target directory exists: {0}")]
	TargetDirectoryExists(PathBuf),
	#[error(transparent)]
	UnknownOSDItem(UnknownOSDItem),
}

#[derive(Debug, Error, From)]
pub enum GenerateOverlayVideoError {
	#[error(transparent)]
	FrameReadError(ReadError),
	#[error("target video file exists: {0}")]
	TargetVideoFileExists(PathBuf),
	#[error("output video file extension needs to be .{expected_extension}")]
	OutputVideoFileExtension { expected_extension: &'static str },
	#[error(transparent)]
	FailedSpawningFFMpegProcess(ffmpeg::SpawnError),
	#[error("failed sending OSD frames to ffmpeg process: {0}")]
	FailedSendingOSDFramesToFFMpeg(IOError),
	#[error(transparent)]
	FFMpegExitedWithError(ffmpeg::ProcessError),
	#[error(transparent)]
	UnknownOSDItem(UnknownOSDItem),
	#[error(transparent)]
	WriteToFileError(TouchError),
}

impl From<SendFramesToFFMpegError> for GenerateOverlayVideoError {
	fn from(error: SendFramesToFFMpegError) -> Self {
		match error {
			SendFramesToFFMpegError::PipeError(error) => Self::FailedSendingOSDFramesToFFMpeg(error),
			SendFramesToFFMpegError::UnknownOSDItem(error) => Self::UnknownOSDItem(error),
			SendFramesToFFMpegError::FFMpegExitedWithError(error) => Self::FFMpegExitedWithError(error),
		}
	}
}

fn best_settings_for_requested_scaling(
	osd_kind: super::Kind,
	scaling: &Scaling,
) -> Result<(Dimensions, tile::Kind, Option<TileDimensions>), DrawFrameOverlayError> {
	Ok(match *scaling {
		Scaling::No { target_resolution } => {
			match target_resolution {
				// no scaling requested but target resolution provided: use the tile kind best matching the target
				// resolution
				Some(target_resolution) => {
					let tile_kind = osd_kind
						.best_kind_of_tiles_to_use_without_scaling(target_resolution.dimensions())
						.map_err(|error| {
							let VideoResolutionTooSmallError {
								osd_kind,
								video_resolution,
							} = error;
							DrawFrameOverlayError::VideoResolutionTooSmallError {
								osd_kind,
								video_resolution,
							}
						})?;
					(osd_kind.dimensions_pixels_for_tile_kind(tile_kind), tile_kind, None)
				},

				// no target resolution specified so use the native tile kind for the OSD kind
				None => (osd_kind.dimensions_pixels(), osd_kind.tile_kind(), None),
			}
		},

		Scaling::Yes {
			min_margins,
			target_resolution,
		} => {
			let max_resolution = VideoResolution::new(
				target_resolution.dimensions().width - 2 * min_margins.horizontal(),
				target_resolution.dimensions().height - 2 * min_margins.vertical(),
			);
			let (tile_kind, tile_dimensions, overlay_dimensions) =
				osd_kind.best_kind_of_tiles_to_use_with_scaling(max_resolution);
			(overlay_dimensions, tile_kind, Some(tile_dimensions))
		},

		Scaling::Auto {
			min_margins,
			min_resolution,
			target_resolution,
		} => {
			let (overlay_resolution, tile_kind, tile_scaling) =

                // check results without scaling
                match best_settings_for_requested_scaling(osd_kind, &Scaling::No { target_resolution: Some(target_resolution) }) {

                    // no scaling is possible
                    Ok(values) => {
                        let (overlay_dimensions, _, _) = values;
                        let (margin_width, margin_height) = crate::video::margins(target_resolution.dimensions(), overlay_dimensions);
						#[allow(clippy::cast_possible_wrap)]
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

			let tile_scaling_yes_no = match tile_scaling {
				Some(_) => "yes",
				None => "no",
			};
			log::info!(
				"calculated best approach: tile kind: {tile_kind} - scaling: {tile_scaling_yes_no} - overlay \
				 resolution: {overlay_resolution}"
			);

			(overlay_resolution, tile_kind, tile_scaling)
		},
	})
}

#[derive(CopyGetters)]
pub struct Generator<'a> {
	osd_file_frames: OSDFileSortedFrames,
	font_variant: FontVariant,
	tile_images: Vec<tile::Image>,
	hidden_regions: &'a [Region],
	hidden_items: Vec<&'a str>,

	#[getset(get_copy = "pub")]
	frame_dimensions: Dimensions,
}

impl<'a> Generator<'a> {
	/// Creates a new `Generator` instance for rendering OSD frames.
	///
	/// # Errors
	/// Returns `DrawFrameOverlayError` if the OSD file is empty or if there are issues loading the
	/// font or determining the best settings for scaling.
	pub fn new(
		osd_file_frames: OSDFileSortedFrames,
		font_variant: FontVariant,
		font_dir: &FontDir,
		font_ident: Option<Option<&str>>,
		scaling: Scaling,
		hide_regions: &'a [Region],
		hide_items: &'a [String],
	) -> Result<Self, DrawFrameOverlayError> {
		if osd_file_frames.is_empty() {
			return Err(DrawFrameOverlayError::OSDFileIsEmpty);
		}

		let (overlay_resolution, tile_kind, tile_scaling) =
			best_settings_for_requested_scaling(osd_file_frames.kind(), &scaling)?;

		let Some(highest_used_tile_index) = osd_file_frames.highest_used_tile_index() else {
			unreachable!();
		};
		let tiles = match font_ident {
			Some(font_ident) => font_dir.load_with_fallback(tile_kind, font_ident, highest_used_tile_index)?,
			None => font_dir.load_variant_with_fallback(
				tile_kind,
				&osd_file_frames.font_variant(),
				highest_used_tile_index,
			)?,
		};

		let tile_images = match tile_scaling {
			Some(tile_dimensions) => tiles.as_slice().resized_tiles_par_with_progress(tile_dimensions),
			None => tiles.into_iter().map(|tile| tile.image().clone()).collect(),
		};

		if let Scaling::No {
			target_resolution: Some(target_resolution),
		} = scaling
		{
			let overlay_width_scale =
				f64::from(overlay_resolution.width) / f64::from(target_resolution.dimensions().width);
			let overlay_height_scale =
				f64::from(overlay_resolution.height) / f64::from(target_resolution.dimensions().height);
			let overlay_res_scale = f64::midpoint(overlay_width_scale, overlay_height_scale);

			if overlay_res_scale < 0.8 {
				log::warn!(
					"without scaling the overlay resolution is much smaller than the target video resolution, \
					 consider using scaling for better results"
				);
			}
		}

		Self::check_osd_file_frames_tile_indices(&osd_file_frames, &tile_images);

		let hidden_items = hide_items.iter().map(String::as_str).collect();

		Ok(Self {
			osd_file_frames,
			tile_images,
			frame_dimensions: overlay_resolution,
			hidden_regions: hide_regions,
			hidden_items,
			font_variant,
		})
	}

	fn check_osd_file_frames_tile_indices(osd_file_frames: &OSDFileSortedFrames, tile_images: &[tile::Image]) {
		let mut invalid_tile_indices = vec![];
		for osd_frame in osd_file_frames.frames() {
			for tile_index in osd_frame.tile_indices().iter() {
				if *tile_index as usize > tile_images.len() - 1 {
					invalid_tile_indices.push(*tile_index);
				}
			}
		}
		if !invalid_tile_indices.is_empty() {
			let invalid_tile_indices_str = invalid_tile_indices
				.iter()
				.map(u16::to_string)
				.unique()
				.collect::<Vec<_>>()
				.join(", ");
			log::warn!(
				"the OSD file contains invalid tile indices, it is probably corrupted or the font you are trying to \
				 render this OSD file does not have that many tiles: {invalid_tile_indices_str}"
			);
		}
	}

	fn draw_frame(&self, osd_file_frame: &OSDFileFrame) -> Result<Frame, UnknownOSDItem> {
		osd_file_frame.draw_overlay_frame(
			self.frame_dimensions,
			self.font_variant,
			&self.tile_images,
			self.hidden_regions,
			&self.hidden_items,
		)
	}

	/// Saves rendered overlay frames to the specified directory.
	///
	/// # Errors
	/// Returns `SaveFramesToDirError` if there are issues creating the directory, reading frames,
	/// writing images, or if there are no frames to write.
	///
	/// # Panics
	/// Panics if the provided path cannot be absolutized.
	pub fn save_frames_to_dir<P: AsRef<Path> + std::marker::Sync>(
		&mut self,
		start: Option<Timestamp>,
		end: Option<Timestamp>,
		path: P,
		frame_shift: i32,
	) -> Result<(), SaveFramesToDirError> {
		if path.as_ref().exists() {
			return Err(SaveFramesToDirError::TargetDirectoryExists(path.as_ref().to_path_buf()));
		}

		create_path(&path)?;
		log::info!(
			"generating overlay frames and saving into directory: {}",
			path.as_ref().to_string_lossy()
		);

		let first_video_frame = start.start_overlay_frame_count();
		let last_video_frame = end.end_overlay_frame_index();

		let osd_file_frames_slice = self
			.osd_file_frames
			.select_slice(first_video_frame, last_video_frame, frame_shift);
		if osd_file_frames_slice.is_empty() {
			return Err(SaveFramesToDirError::NoFrameToWrite);
		}

		let iter = osd_file_frames_slice.video_frames_rel_index_par_iter(EndOfFramesAction::ContinueToLastVideoFrame);
		let frame_count = iter.len();

		#[allow(clippy::literal_string_with_formatting_args)]
		let Ok(progress_style) = ProgressStyle::with_template("{wide_bar} {pos:>6}/{len}") else {
			unreachable!();
		};
		let progress_bar = ProgressBar::new(frame_count as u64).with_style(progress_style);
		progress_bar.enable_steady_tick(std::time::Duration::new(0, 100_000_000));

		let abs_output_dir_path = path.as_ref().absolutize().unwrap();

		iter.progress_with(progress_bar).try_for_each(|item| {
			use crate::osd::file::sorted_frames::VideoFramesRelIndexIterItem as VFRIII;
			match item {
				VFRIII::Existing { rel_index, frame } => {
					log::debug!("existing {rel_index}");
					let frame_image = self.draw_frame(frame)?;
					frame_image.write_image_file(make_overlay_frame_file_path(&path, rel_index))?;
				},
				VFRIII::FirstNonExisting => {
					log::debug!("first non existing");
					let frame_0_path = make_overlay_frame_file_path(&path, 0);
					Frame::new(self.frame_dimensions).write_image_file(frame_0_path)?;
				},
				VFRIII::NonExisting {
					prev_rel_index,
					rel_index,
				} => {
					log::debug!("non existing {rel_index} -> {prev_rel_index}");
					let prev_path = make_overlay_frame_file_path(&abs_output_dir_path, prev_rel_index);
					let link_path = make_overlay_frame_file_path(&path, rel_index);
					fs_err::os::unix::fs::symlink(prev_path, link_path).map_err(SaveFramesToDirError::SymlinkError)?;
				},
			}
			Ok::<(), SaveFramesToDirError>(())
		})?;

		log::info!("overlay frames generation completed: {frame_count} frame files written");
		Ok(())
	}

	/// Generates an overlay video from the OSD frames.
	///
	/// # Errors
	/// Returns `GenerateOverlayVideoError` if there are issues reading frames, writing the output
	/// video file, or if the output file already exists.
	///
	/// # Panics
	/// It does not panic.
	#[allow(clippy::too_many_arguments)]
	pub async fn generate_overlay_video<P: AsRef<Path>>(
		&mut self,
		codec: OverlayVideoCodec,
		start: Option<Timestamp>,
		end: Option<Timestamp>,
		output_video_path: P,
		frame_shift: i32,
		overwrite_output: bool,
		ffmpeg_priority: Option<i32>,
	) -> Result<(), GenerateOverlayVideoError> {
		let output_video_path = output_video_path.as_ref();

		if !matches!(output_video_path.extension(), Some(extension) if extension == codec.file_extension()) {
			return Err(GenerateOverlayVideoError::OutputVideoFileExtension {
				expected_extension: codec.file_extension(),
			});
		}

		if !overwrite_output && output_video_path.exists() {
			return Err(GenerateOverlayVideoError::TargetVideoFileExists(
				output_video_path.to_path_buf(),
			));
		}

		file::touch(output_video_path)?;
		let guard = crate::file::OutputFileGuard::new(output_video_path);

		log::info!("generating overlay video: {}", output_video_path.to_string_lossy());

		let frames_iter = self.iter_advanced(
			start.start_overlay_frame_count(),
			end.end_overlay_frame_index(),
			frame_shift,
		);
		let frame_count = frames_iter.len();

		let mut ffmpeg_command = ffmpeg::CommandBuilder::default();

		let packed_alpha_layout = codec.packed_alpha_layout(self.frame_dimensions);
		let input_dimensions = packed_alpha_layout.map_or(self.frame_dimensions, PackedAlphaLayout::packed_dimensions);
		ffmpeg_command
			.add_stdin_input(input_dimensions, 60)
			.unwrap()
			.set_output_file(output_video_path)
			.set_overwrite_output_file(true);
		configure_overlay_video_output(&mut ffmpeg_command, codec, packed_alpha_layout);

		let spawn_options = ffmpeg::SpawnOptions::default()
			.with_progress(frame_count as u64)
			.with_priority(ffmpeg_priority);
		let ffmpeg_process = ffmpeg_command.build().unwrap().spawn(&spawn_options)?;

		frames_iter
			.send_frames_to_ffmpeg_and_wait(ffmpeg_process, packed_alpha_layout)
			.await?;

		log::info!("overlay video generation completed: {frame_count} frames");
		guard.defuse();
		Ok(())
	}

	/// Generate the overlay video with a progress callback instead of the indicatif TTY bar.
	///
	/// # Errors
	/// Returns [`GenerateOverlayVideoError`] if generation fails.
	///
	/// # Panics
	/// Panics if the ffmpeg command cannot be built.
	#[allow(clippy::too_many_arguments)]
	pub async fn generate_overlay_video_with_progress<P, F>(
		&mut self,
		codec: OverlayVideoCodec,
		start: Option<Timestamp>,
		end: Option<Timestamp>,
		output_video_path: P,
		frame_shift: i32,
		overwrite_output: bool,
		ffmpeg_priority: Option<i32>,
		callback: F,
	) -> Result<(), GenerateOverlayVideoError>
	where
		P: AsRef<Path>,
		F: Fn(u64, u64) + Send + 'static,
	{
		let output_video_path = output_video_path.as_ref();

		if !matches!(output_video_path.extension(), Some(extension) if extension == codec.file_extension()) {
			return Err(GenerateOverlayVideoError::OutputVideoFileExtension {
				expected_extension: codec.file_extension(),
			});
		}

		if !overwrite_output && output_video_path.exists() {
			return Err(GenerateOverlayVideoError::TargetVideoFileExists(
				output_video_path.to_path_buf(),
			));
		}

		file::touch(output_video_path)?;
		let guard = crate::file::OutputFileGuard::new(output_video_path);

		log::info!("generating overlay video: {}", output_video_path.to_string_lossy());

		let frames_iter = self.iter_advanced(
			start.start_overlay_frame_count(),
			end.end_overlay_frame_index(),
			frame_shift,
		);
		let frame_count = frames_iter.len();

		let mut ffmpeg_command = ffmpeg::CommandBuilder::default();

		let packed_alpha_layout = codec.packed_alpha_layout(self.frame_dimensions);
		let input_dimensions = packed_alpha_layout.map_or(self.frame_dimensions, PackedAlphaLayout::packed_dimensions);
		ffmpeg_command
			.add_stdin_input(input_dimensions, 60)
			.unwrap()
			.set_output_file(output_video_path)
			.set_overwrite_output_file(true);
		configure_overlay_video_output(&mut ffmpeg_command, codec, packed_alpha_layout);

		let spawn_options = ffmpeg::SpawnOptionsWithCallback::default()
			.with_callback(frame_count as u64, Box::new(callback))
			.with_priority(ffmpeg_priority);
		let ffmpeg_process = ffmpeg_command.build().unwrap().spawn_with_callback(spawn_options)?;

		frames_iter
			.send_frames_to_ffmpeg_callback_and_wait(ffmpeg_process, packed_alpha_layout)
			.await?;

		log::info!("overlay video generation completed: {frame_count} frames");
		guard.defuse();
		Ok(())
	}

	#[must_use]
	pub fn iter(&self) -> FramesIter<'_> {
		self.into_iter()
	}

	#[must_use]
	pub fn iter_advanced(&self, first_frame: u32, last_frame: Option<u32>, frame_shift: i32) -> FramesIter<'_> {
		FramesIter {
			frame_dimensions: self.frame_dimensions,
			font_variant: self.font_variant,
			tile_images: &self.tile_images,
			vframes_iter: self
				.osd_file_frames
				.video_frames_iter(first_frame, last_frame, frame_shift),
			hidden_regions: self.hidden_regions,
			hidden_items: &self.hidden_items,
			prev_frame: Frame::new(self.frame_dimensions),
		}
	}
}

impl<'a> IntoIterator for &'a Generator<'a> {
	type IntoIter = FramesIter<'a>;
	type Item = Result<Frame, UnknownOSDItem>;

	fn into_iter(self) -> Self::IntoIter {
		self.iter_advanced(0, None, 0)
	}
}

#[derive(Debug, Error, From)]
pub enum SendFramesToFFMpegError {
	#[error("error sending overlay frames to FFMpeg: pipe error: {0}")]
	PipeError(io::Error),
	#[error(transparent)]
	UnknownOSDItem(UnknownOSDItem),
	#[error(transparent)]
	FFMpegExitedWithError(ffmpeg::ProcessError),
}

#[derive(CopyGetters)]
pub struct FramesIter<'a> {
	#[getset(get_copy = "pub")]
	frame_dimensions: Dimensions,
	font_variant: FontVariant,
	tile_images: &'a [tile::Image],
	vframes_iter: VideoFramesIter<'a>,
	hidden_regions: &'a [Region],
	hidden_items: &'a [&'a str],
	prev_frame: Frame,
}

impl FramesIter<'_> {
	/// Sends rendered frames to the provided ffmpeg process.
	///
	/// # Errors
	/// Returns `SendFramesToFFMpegError` if there are issues writing to ffmpeg's stdin or if there
	/// are unknown OSD items.
	///
	/// # Panics
	/// Panics if the stdin of the ffmpeg process has already been taken.
	pub fn send_frames_to_ffmpeg(
		&mut self,
		ffmpeg_process: &mut ffmpeg::Process,
		packed_alpha_layout: Option<PackedAlphaLayout>,
	) -> Result<(), SendFramesToFFMpegError> {
		let mut ffmpeg_stdin = ffmpeg_process.take_stdin().unwrap();
		for osd_frame_image in self {
			let mut osd_frame_image = osd_frame_image?;
			if let Some(layout) = packed_alpha_layout {
				osd_frame_image = osd_frame_image.pack_alpha(layout);
			}
			ffmpeg_stdin.write_all(osd_frame_image.as_raw())?;
		}
		drop(ffmpeg_stdin);
		Ok(())
	}

	/// Sends rendered frames to the provided ffmpeg process and waits for it to finish.
	///
	/// # Errors
	/// Returns `SendFramesToFFMpegError` if there are issues writing to ffmpeg's stdin, if there
	/// are unknown OSD items, or if ffmpeg exits with an error
	pub async fn send_frames_to_ffmpeg_and_wait(
		mut self,
		mut ffmpeg_process: ffmpeg::Process,
		packed_alpha_layout: Option<PackedAlphaLayout>,
	) -> Result<(), SendFramesToFFMpegError> {
		let send_result = self.send_frames_to_ffmpeg(&mut ffmpeg_process, packed_alpha_layout);

		ffmpeg_process.wait().await?;
		send_result?;

		Ok(())
	}

	/// Sends rendered frames to a [`ffmpeg::ProcessWithCallback`] and waits for it to finish.
	///
	/// Uses async I/O to write frames so it is safe to call from an async context.
	///
	/// # Errors
	/// Returns `SendFramesToFFMpegError` if there are issues writing to ffmpeg's stdin, if there
	/// are unknown OSD items, or if ffmpeg exits with an error.
	///
	/// # Panics
	/// Panics if the stdin of the ffmpeg process has already been taken.
	pub async fn send_frames_to_ffmpeg_callback_and_wait(
		self,
		mut ffmpeg_process: ffmpeg::ProcessWithCallback,
		packed_alpha_layout: Option<PackedAlphaLayout>,
	) -> Result<(), SendFramesToFFMpegError> {
		let mut ffmpeg_stdin = ffmpeg_process.take_stdin().unwrap();
		let mut frame_error: Option<SendFramesToFFMpegError> = None;
		for osd_frame_image in self {
			match osd_frame_image {
				Ok(mut frame) => {
					if let Some(layout) = packed_alpha_layout {
						frame = frame.pack_alpha(layout);
					}
					if let Err(e) = ffmpeg_stdin.write_all(frame.as_raw()).await {
						frame_error = Some(SendFramesToFFMpegError::PipeError(e));
						break;
					}
				},
				Err(e) => {
					frame_error = Some(SendFramesToFFMpegError::UnknownOSDItem(e));
					break;
				},
			}
		}
		drop(ffmpeg_stdin);
		ffmpeg_process.wait().await?;
		if let Some(e) = frame_error {
			return Err(e);
		}
		Ok(())
	}
}

impl Iterator for FramesIter<'_> {
	type Item = Result<Frame, UnknownOSDItem>;

	fn next(&mut self) -> Option<Self::Item> {
		match self.vframes_iter.next()? {
			Some(osd_file_frame) => {
				let frame = match osd_file_frame.draw_overlay_frame(
					self.frame_dimensions,
					self.font_variant,
					self.tile_images,
					self.hidden_regions,
					self.hidden_items,
				) {
					Ok(frame) => frame,
					Err(error) => return Some(Err(error)),
				};
				self.prev_frame = frame.clone();
				Some(Ok(frame))
			},
			None => Some(Ok(self.prev_frame.clone())),
		}
	}
}

impl ExactSizeIterator for FramesIter<'_> {
	fn len(&self) -> usize {
		self.vframes_iter.len()
	}
}

#[cfg(test)]
mod tests {
	use image::Rgba;

	use super::{
		Dimensions, Frame, OVERLAY_ALPHA_X_METADATA, OVERLAY_MODE_METADATA, OVERLAY_SIZE_METADATA, OverlayVideoCodec,
		PackedAlphaLayout, configure_overlay_video_output, packed_alpha_quality,
	};
	use crate::{
		ffmpeg::VideoQuality,
		video::{Codec, HwAcceleratedEncoding},
	};

	#[test]
	fn packed_alpha_quality_should_use_each_vaapi_codec_scale() {
		assert!(matches!(
			packed_alpha_quality(Codec::AV1, HwAcceleratedEncoding::Yes),
			VideoQuality::GlobalQuality(90)
		));
		assert!(matches!(
			packed_alpha_quality(Codec::H265, HwAcceleratedEncoding::Yes),
			VideoQuality::GlobalQuality(24)
		));
		assert!(matches!(
			packed_alpha_quality(Codec::H265, HwAcceleratedEncoding::No),
			VideoQuality::ConstantRateFactor(28)
		));
	}

	#[test]
	fn packed_alpha_layout_should_align_the_alpha_plane() {
		let layout = PackedAlphaLayout::new(Dimensions::new(1200, 648));

		assert_eq!(layout.alpha_x(), 1216);
		assert_eq!(layout.packed_dimensions(), Dimensions::new(2416, 648));
	}

	#[test]
	fn packed_alpha_layout_should_not_pad_aligned_widths() {
		let layout = PackedAlphaLayout::new(Dimensions::new(1280, 720));

		assert_eq!(layout.alpha_x(), 1280);
		assert_eq!(layout.packed_dimensions(), Dimensions::new(2560, 720));
	}

	#[test]
	fn packed_alpha_should_split_color_and_alpha_planes() {
		let mut frame = Frame::new(Dimensions::new(3, 1));
		frame.image.put_pixel(1, 0, Rgba([255, 255, 255, 128]));
		frame.image.put_pixel(2, 0, Rgba([10, 20, 30, 255]));

		let packed = frame.pack_alpha(PackedAlphaLayout::new(frame.dimensions()));

		assert_eq!(packed.dimensions(), Dimensions::new(67, 1));
		assert_eq!(packed.image.get_pixel(0, 0), &Rgba([0, 0, 0, 255]));
		assert_eq!(packed.image.get_pixel(1, 0), &Rgba([255, 255, 255, 255]));
		assert_eq!(packed.image.get_pixel(2, 0), &Rgba([10, 20, 30, 255]));
		assert_eq!(packed.image.get_pixel(3, 0), &Rgba([0, 0, 0, 0]));
		assert_eq!(packed.image.get_pixel(64, 0), &Rgba([0, 0, 0, 255]));
		assert_eq!(packed.image.get_pixel(65, 0), &Rgba([128, 128, 128, 255]));
		assert_eq!(packed.image.get_pixel(66, 0), &Rgba([255, 255, 255, 255]));
	}

	#[test]
	fn packed_alpha_layout_should_round_trip_through_metadata() {
		let layout = PackedAlphaLayout::new(Dimensions::new(1200, 648));

		assert_eq!(
			PackedAlphaLayout::from_metadata(&layout.overlay_dimensions().to_string(), &layout.alpha_x().to_string()),
			Some(layout)
		);
	}

	#[test]
	fn packed_alpha_layout_should_reject_invalid_metadata() {
		for (size, alpha_x) in [
			("1200x648", "1199"),
			("1200", "1216"),
			("0x648", "1216"),
			("1200x648", "-1"),
			("1200x648x1", "1216"),
		] {
			assert_eq!(
				PackedAlphaLayout::from_metadata(size, alpha_x),
				None,
				"{size} {alpha_x}"
			);
		}
	}

	#[test]
	fn packed_alpha_unpack_filter_should_crop_explicit_planes() {
		assert_eq!(
			PackedAlphaLayout::new(Dimensions::new(1200, 648)).unpack_filter("vid2", "osd"),
			"[vid2]split[osd_packed_color][osd_packed_alpha];[osd_packed_color]crop=1200:648:0:0[osd_color];\
			 [osd_packed_alpha]crop=1200:648:1216:0,format=gray[osd_alpha];[osd_color][osd_alpha]alphamerge[osd]"
		);
	}

	#[test]
	fn packed_alpha_codec_should_be_the_mp4_default() {
		assert!(OverlayVideoCodec::default().is_packed_alpha());
		assert_eq!(OverlayVideoCodec::PackedAlpha.file_extension(), "mp4");
	}

	#[test]
	fn packed_alpha_command_should_include_mode_and_geometry_metadata() {
		let mut command = crate::ffmpeg::CommandBuilder::default();
		configure_overlay_video_output(
			&mut command,
			OverlayVideoCodec::PackedAlpha,
			Some(PackedAlphaLayout::new(Dimensions::new(1200, 648))),
		);

		assert_eq!(
			command.args(),
			&vec![
				"-an".to_owned(),
				"-movflags".to_owned(),
				"use_metadata_tags".to_owned(),
				"-metadata".to_owned(),
				format!("{OVERLAY_MODE_METADATA}=packed-alpha"),
				"-metadata".to_owned(),
				format!("{OVERLAY_SIZE_METADATA}=1200x648"),
				"-metadata".to_owned(),
				format!("{OVERLAY_ALPHA_X_METADATA}=1216"),
			]
		);
	}
}
