use std::{
	fmt::{Display, Formatter},
	io::{self, Error as IOError, Write},
	path::{Path, PathBuf},
	str::FromStr,
};

use derive_more::{Deref, From};
use getset::{CopyGetters, Getters};
use image::{GenericImage, ImageBuffer, ImageResult, Rgba};
use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};
use itertools::Itertools;
use path_absolutize::Absolutize;
use rayon::prelude::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
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
		FrameIndex as VideoFrameIndex,
		resolution::Resolution as VideoResolution,
		timestamp::{StartEndOverlayFrameIndex, Timestamp},
	},
};

pub type Dimensions = GenericDimensions<u32>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChromaKeyColor([u8; 3]);

/// Configures whether a chroma key is chosen automatically or explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChromaKeyColorSelection {
	/// Select the color furthest from visible glyph colors.
	#[default]
	Automatic,
	/// Use the supplied RGB color.
	Explicit(ChromaKeyColor),
}

impl Display for ChromaKeyColorSelection {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Automatic => formatter.write_str("auto"),
			Self::Explicit(color) => color.fmt(formatter),
		}
	}
}

impl FromStr for ChromaKeyColorSelection {
	type Err = String;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
		if value.eq_ignore_ascii_case("auto") {
			Ok(Self::Automatic)
		} else {
			value.parse().map(Self::Explicit)
		}
	}
}

impl ChromaKeyColor {
	#[must_use]
	pub const fn legacy_magenta() -> Self {
		Self([255, 0, 255])
	}

	#[must_use]
	pub const fn rgb(self) -> [u8; 3] {
		self.0
	}

	#[must_use]
	pub fn ffmpeg_color(self) -> String {
		format!("0x{:02X}{:02X}{:02X}", self.0[0], self.0[1], self.0[2])
	}

	fn furthest_from_visible_colors(images: &[tile::Image]) -> Self {
		let mut colors = images
			.iter()
			.flat_map(tile::Image::pixels)
			.filter(|pixel| pixel[3] != 0)
			.map(|pixel| [pixel[0], pixel[1], pixel[2]])
			.collect::<Vec<_>>();
		colors.sort_unstable();
		colors.dedup();

		Self::furthest_from_colors(&colors)
	}

	fn furthest_from_colors(colors: &[[u8; 3]]) -> Self {
		if colors.is_empty() {
			return Self::default();
		}

		let (_, color) = (u8::MIN..=u8::MAX)
			.into_par_iter()
			.map(|red| {
				let mut furthest = (0, [red, 0, 0]);
				for green in u8::MIN..=u8::MAX {
					for blue in u8::MIN..=u8::MAX {
						let color = [red, green, blue];
						let closest_distance = colors
							.iter()
							.map(|glyph_color| color_distance_squared(color, *glyph_color))
							.min()
							.unwrap_or_default();
						let candidate = (closest_distance, color);
						if is_further_color(candidate, furthest) {
							furthest = candidate;
						}
					}
				}
				furthest
			})
			.reduce(
				|| (0, [0, 0, 0]),
				|first, second| {
					if is_further_color(first, second) { first } else { second }
				},
			);

		Self(color)
	}
}

impl Default for ChromaKeyColor {
	fn default() -> Self {
		Self([0, 0, 64])
	}
}

fn color_distance_squared(first: [u8; 3], second: [u8; 3]) -> u32 {
	first
		.into_iter()
		.zip(second)
		.map(|(first, second)| {
			let difference = i32::from(first) - i32::from(second);
			let difference = difference.unsigned_abs();
			difference * difference
		})
		.sum()
}

fn is_further_color(first: (u32, [u8; 3]), second: (u32, [u8; 3])) -> bool {
	first.0 > second.0 || (first.0 == second.0 && first.1 < second.1)
}

impl Display for ChromaKeyColor {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
		write!(formatter, "#{:02X}{:02X}{:02X}", self.0[0], self.0[1], self.0[2])
	}
}

impl FromStr for ChromaKeyColor {
	type Err = String;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
		let value = value.strip_prefix('#').unwrap_or(value);
		if !value.is_ascii() || value.len() != 6 {
			return Err("key color must be a six-digit RGB value such as #FF00FF".to_owned());
		}
		let red = u8::from_str_radix(&value[0..2], 16)
			.map_err(|_| "key color must be a six-digit RGB value such as #FF00FF".to_owned())?;
		let green = u8::from_str_radix(&value[2..4], 16)
			.map_err(|_| "key color must be a six-digit RGB value such as #FF00FF".to_owned())?;
		let blue = u8::from_str_radix(&value[4..6], 16)
			.map_err(|_| "key color must be a six-digit RGB value such as #FF00FF".to_owned())?;
		Ok(Self([red, green, blue]))
	}
}

#[derive(Debug, Error)]
#[error("rendered OSD pixels use the chroma key color {0}")]
pub struct ChromaKeyCollisionError(ChromaKeyColor);

const MIN_CHROMA_KEY_COLLISION_ALPHA: u8 = 2;

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

	/// Replaces transparent pixels with an opaque key color for chroma-key encoding.
	///
	/// # Errors
	/// Returns [`ChromaKeyCollisionError`] when a visible rendered OSD pixel uses the selected key
	/// color.
	pub fn apply_chroma_key(&mut self, key_color: ChromaKeyColor) -> Result<(), ChromaKeyCollisionError> {
		let [key_red, key_green, key_blue] = key_color.rgb();
		for pixel in self.image.pixels_mut() {
			if pixel[3] == 0 {
				*pixel = Rgba([key_red, key_green, key_blue, 255]);
			} else if pixel[3] >= MIN_CHROMA_KEY_COLLISION_ALPHA
				&& [pixel[0], pixel[1], pixel[2]] == [key_red, key_green, key_blue]
			{
				return Err(ChromaKeyCollisionError(key_color));
			} else {
				pixel[0] = blend_chroma_component(pixel[0], key_red, pixel[3]);
				pixel[1] = blend_chroma_component(pixel[1], key_green, pixel[3]);
				pixel[2] = blend_chroma_component(pixel[2], key_blue, pixel[3]);
				pixel[3] = 255;
			}
		}
		Ok(())
	}
}

fn blend_chroma_component(component: u8, key_component: u8, alpha: u8) -> u8 {
	let alpha = u16::from(alpha);
	let value = (u16::from(component) * alpha + u16::from(key_component) * (255 - alpha)) / 255;
	match u8::try_from(value) {
		Ok(value) => value,
		Err(_) => unreachable!("blending two u8 values cannot produce a value greater than u8::MAX"),
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
	#[default]
	ChromaKey,
	Vp8,
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
			Self::ChromaKey => "mp4",
			Self::Vp8 | Self::Vp9 => "webm",
		}
	}

	#[must_use]
	pub fn is_chroma_key(self) -> bool {
		matches!(self, Self::ChromaKey)
	}

	#[must_use]
	pub fn params(&self) -> Option<OverlayVideoCodecParams> {
		use OverlayVideoCodec as OVC;
		use OverlayVideoCodecParams as OVCP;
		Some(match self {
			OVC::ChromaKey => return None,
			OVC::Vp8 => OVCP::new("libvpx", Some("1M"), Some(40), &["-auto-alt-ref", "0"]),
			OVC::Vp9 => OVCP::new("libvpx-vp9", Some("0"), Some(40), &[]),
		})
	}
}

pub const CHROMA_KEY_MODE_METADATA: &str = "hd_fpv_overlay_mode";
pub const CHROMA_KEY_COLOR_METADATA: &str = "hd_fpv_overlay_key_color";
pub const CHROMA_KEY_FILTER_METADATA: &str = "hd_fpv_overlay_filter";
const CHROMA_KEY_BITRATE_CAP: &str = "4M";
const CHROMA_KEY_SIMILARITY: &str = "0.05";
const LEGACY_CHROMA_KEY_SIMILARITY: &str = "0.01";
const CHROMA_KEY_BLEND: &str = "0.0";

#[must_use]
pub fn chroma_key_filter(key_color: ChromaKeyColor) -> String {
	format!(
		"chromakey={}:{}:{}",
		key_color.ffmpeg_color(),
		CHROMA_KEY_SIMILARITY,
		CHROMA_KEY_BLEND
	)
}

#[must_use]
pub fn is_supported_chroma_key_filter(key_color: ChromaKeyColor, filter: &str) -> bool {
	[CHROMA_KEY_SIMILARITY, LEGACY_CHROMA_KEY_SIMILARITY]
		.into_iter()
		.any(|similarity| {
			filter
				== format!(
					"chromakey={}:{}:{}",
					key_color.ffmpeg_color(),
					similarity,
					CHROMA_KEY_BLEND
				)
		})
}

fn configure_overlay_video_output(
	ffmpeg_command: &mut ffmpeg::CommandBuilder,
	codec: OverlayVideoCodec,
	key_color: ChromaKeyColor,
) {
	if codec.is_chroma_key() {
		let (video_codec, hw_acceleration) = crate::video::default_video_codec(false);
		let quality = match hw_acceleration {
			crate::video::HwAcceleratedEncoding::Yes => VideoQuality::GlobalQuality(90),
			crate::video::HwAcceleratedEncoding::No => VideoQuality::ConstantRateFactor(28),
		};
		let filter = chroma_key_filter(key_color);
		ffmpeg_command
			.set_output_video_settings(
				Some(video_codec.ffmpeg_string(hw_acceleration.is_yes())),
				Some(CHROMA_KEY_BITRATE_CAP),
				Some(quality),
			)
			.add_arg("-an")
			.add_arg("-movflags")
			.add_arg("use_metadata_tags")
			.add_arg("-metadata")
			.add_arg(&format!("{CHROMA_KEY_MODE_METADATA}=chroma-key"))
			.add_arg("-metadata")
			.add_arg(&format!("{CHROMA_KEY_COLOR_METADATA}={key_color}"))
			.add_arg("-metadata")
			.add_arg(&format!("{CHROMA_KEY_FILTER_METADATA}={filter}"));
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
	ChromaKeyCollision(ChromaKeyCollisionError),
	#[error(transparent)]
	WriteToFileError(TouchError),
}

impl From<SendFramesToFFMpegError> for GenerateOverlayVideoError {
	fn from(error: SendFramesToFFMpegError) -> Self {
		match error {
			SendFramesToFFMpegError::PipeError(error) => Self::FailedSendingOSDFramesToFFMpeg(error),
			SendFramesToFFMpegError::UnknownOSDItem(error) => Self::UnknownOSDItem(error),
			SendFramesToFFMpegError::ChromaKeyCollision(error) => Self::ChromaKeyCollision(error),
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

	/// Selects the RGB color with the greatest distance from every visible loaded glyph color.
	#[must_use]
	pub fn automatic_chroma_key_color(&self) -> ChromaKeyColor {
		ChromaKeyColor::furthest_from_visible_colors(&self.tile_images)
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
		key_color: ChromaKeyColor,
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

		ffmpeg_command
			.add_stdin_input(self.frame_dimensions, 60)
			.unwrap()
			.set_output_file(output_video_path)
			.set_overwrite_output_file(true);
		configure_overlay_video_output(&mut ffmpeg_command, codec, key_color);

		let spawn_options = ffmpeg::SpawnOptions::default()
			.with_progress(frame_count as u64)
			.with_priority(ffmpeg_priority);
		let ffmpeg_process = ffmpeg_command.build().unwrap().spawn(&spawn_options)?;

		frames_iter
			.send_frames_to_ffmpeg_and_wait(ffmpeg_process, codec.is_chroma_key().then_some(key_color))
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
		key_color: ChromaKeyColor,
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

		ffmpeg_command
			.add_stdin_input(self.frame_dimensions, 60)
			.unwrap()
			.set_output_file(output_video_path)
			.set_overwrite_output_file(true);
		configure_overlay_video_output(&mut ffmpeg_command, codec, key_color);

		let spawn_options = ffmpeg::SpawnOptionsWithCallback::default()
			.with_callback(frame_count as u64, Box::new(callback))
			.with_priority(ffmpeg_priority);
		let ffmpeg_process = ffmpeg_command.build().unwrap().spawn_with_callback(spawn_options)?;

		frames_iter
			.send_frames_to_ffmpeg_callback_and_wait(ffmpeg_process, codec.is_chroma_key().then_some(key_color))
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
	ChromaKeyCollision(ChromaKeyCollisionError),
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
		key_color: Option<ChromaKeyColor>,
	) -> Result<(), SendFramesToFFMpegError> {
		let mut ffmpeg_stdin = ffmpeg_process.take_stdin().unwrap();
		for osd_frame_image in self {
			let mut osd_frame_image = osd_frame_image?;
			if let Some(key_color) = key_color {
				osd_frame_image.apply_chroma_key(key_color)?;
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
		key_color: Option<ChromaKeyColor>,
	) -> Result<(), SendFramesToFFMpegError> {
		let send_result = self.send_frames_to_ffmpeg(&mut ffmpeg_process, key_color);

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
		key_color: Option<ChromaKeyColor>,
	) -> Result<(), SendFramesToFFMpegError> {
		let mut ffmpeg_stdin = ffmpeg_process.take_stdin().unwrap();
		let mut frame_error: Option<SendFramesToFFMpegError> = None;
		for osd_frame_image in self {
			match osd_frame_image {
				Ok(mut frame) => {
					let chroma_key_error = key_color.and_then(|key_color| frame.apply_chroma_key(key_color).err());
					if let Some(error) = chroma_key_error {
						frame_error = Some(SendFramesToFFMpegError::ChromaKeyCollision(error));
						break;
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
		CHROMA_KEY_COLOR_METADATA, CHROMA_KEY_FILTER_METADATA, CHROMA_KEY_MODE_METADATA, ChromaKeyColor,
		ChromaKeyColorSelection, Dimensions, Frame, OverlayVideoCodec, chroma_key_filter,
		configure_overlay_video_output, is_supported_chroma_key_filter,
	};

	#[test]
	fn chroma_key_default_should_use_the_validated_color() {
		assert_eq!(ChromaKeyColor::default().rgb(), [0, 0, 64]);
	}

	#[test]
	fn automatic_chroma_key_color_should_be_furthest_from_visible_glyph_colors() {
		assert_eq!(
			ChromaKeyColor::furthest_from_colors(&[[255, 255, 255]]).rgb(),
			[0, 0, 0]
		);
	}

	#[test]
	fn chroma_key_color_selection_should_parse_automatic_and_explicit_values() {
		assert_eq!(
			"auto".parse::<ChromaKeyColorSelection>().unwrap(),
			ChromaKeyColorSelection::Automatic
		);
		assert_eq!(
			"00FF00".parse::<ChromaKeyColorSelection>().unwrap(),
			ChromaKeyColorSelection::Explicit("00FF00".parse().unwrap())
		);
	}

	#[test]
	fn chroma_key_should_replace_transparent_pixels_and_preserve_rendered_pixels() {
		let mut frame = Frame::new(Dimensions::new(2, 1));
		frame.image.put_pixel(1, 0, Rgba([10, 20, 30, 255]));

		frame.apply_chroma_key(ChromaKeyColor::default()).unwrap();

		assert_eq!(frame.image.into_raw(), vec![0, 0, 64, 255, 10, 20, 30, 255],);
	}

	#[test]
	fn chroma_key_should_reject_rendered_key_color_pixels() {
		let mut frame = Frame::new(Dimensions::new(1, 1));
		frame.image.put_pixel(0, 0, Rgba([0, 0, 64, 255]));

		assert!(frame.apply_chroma_key(ChromaKeyColor::default()).is_err());
	}

	#[test]
	fn chroma_key_should_accept_nearly_transparent_key_color_pixels() {
		let mut frame = Frame::new(Dimensions::new(1, 1));
		frame.image.put_pixel(0, 0, Rgba([255, 0, 255, 1]));

		frame.apply_chroma_key(ChromaKeyColor([255, 0, 255])).unwrap();

		assert_eq!(frame.image.into_raw(), vec![255, 0, 255, 255]);
	}

	#[test]
	fn chroma_key_color_should_reject_non_ascii_input() {
		assert!("€abc".parse::<ChromaKeyColor>().is_err());
	}

	#[test]
	fn chroma_key_should_blend_translucent_pixels_against_the_key_color() {
		let mut frame = Frame::new(Dimensions::new(1, 1));
		frame.image.put_pixel(0, 0, Rgba([255, 255, 255, 128]));

		frame.apply_chroma_key(ChromaKeyColor::default()).unwrap();

		assert_eq!(frame.image.into_raw(), vec![128, 128, 159, 255]);
	}

	#[test]
	fn chroma_key_codec_should_use_mp4_sidecars() {
		assert_eq!(OverlayVideoCodec::ChromaKey.file_extension(), "mp4");
	}

	#[test]
	fn chroma_key_filter_should_use_the_validated_default_color() {
		assert_eq!(
			chroma_key_filter(ChromaKeyColor::default()),
			"chromakey=0x000040:0.05:0.0"
		);
	}

	#[test]
	fn chroma_key_filter_should_accept_legacy_similarity() {
		assert!(is_supported_chroma_key_filter(
			ChromaKeyColor::legacy_magenta(),
			"chromakey=0xFF00FF:0.01:0.0"
		));
	}

	#[test]
	fn chroma_key_command_should_include_mode_key_and_filter_metadata() {
		let mut command = crate::ffmpeg::CommandBuilder::default();
		configure_overlay_video_output(&mut command, OverlayVideoCodec::ChromaKey, ChromaKeyColor::default());

		assert_eq!(
			command.args(),
			&vec![
				"-an".to_owned(),
				"-movflags".to_owned(),
				"use_metadata_tags".to_owned(),
				"-metadata".to_owned(),
				format!("{CHROMA_KEY_MODE_METADATA}=chroma-key"),
				"-metadata".to_owned(),
				format!("{CHROMA_KEY_COLOR_METADATA}=#000040"),
				"-metadata".to_owned(),
				format!(
					"{CHROMA_KEY_FILTER_METADATA}={}",
					chroma_key_filter(ChromaKeyColor::default())
				),
			]
		);
	}
}
