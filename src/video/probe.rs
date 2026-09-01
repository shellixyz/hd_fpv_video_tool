use std::{
	ffi::CStr,
	path::{Path, PathBuf},
};

use ffmpeg::Rational;
use ffmpeg_next as ffmpeg;
use getset::{CopyGetters, Getters};
use thiserror::Error;

use super::resolution::Resolution;

#[derive(Debug, Error)]
pub enum Error {
	#[error("failed initializing ffmpeg: {0}")]
	FFMpegInit(ffmpeg::Error),
	#[error("failed opening video file {file_path}: {error}")]
	FfmpegFileError { file_path: PathBuf, error: ffmpeg::Error },
	#[error("cannot find video stream in file: {0}")]
	CannotFindVideoStream(PathBuf),
}

impl Error {
	pub fn ffmpeg<P: AsRef<Path>>(file_path: P, error: ffmpeg::Error) -> Self {
		Self::FfmpegFileError {
			file_path: file_path.as_ref().to_path_buf(),
			error,
		}
	}
}

#[derive(Debug, Clone, CopyGetters, Getters)]
#[getset(get_copy = "pub")]
pub struct Result {
	frame_count: u64,
	frame_rate: Rational,
	has_audio: bool,
	resolution: Resolution,

	#[getset(skip)]
	#[getset(get = "pub")]
	video_codec: Option<String>,
}

/// Probes the specified video file and retrieves its properties.
///
/// # Errors
/// - Returns `Error::FFMpegInit` if ffmpeg fails to initialize.
/// - Returns `Error::FfmpegFileError` if there is an error opening the video file.
/// - Returns `Error::CannotFindVideoStream` if no video stream is found in the file
pub fn probe<P: AsRef<Path>>(video_file: P) -> std::result::Result<Result, Error> {
	ffmpeg::init().map_err(Error::FFMpegInit)?;
	ffmpeg::log::set_level(ffmpeg::log::Level::Quiet);

	let input = ffmpeg::format::input(&video_file).map_err(|error| Error::ffmpeg(&video_file, error))?;

	let has_audio = input.streams().best(ffmpeg::media::Type::Audio).is_some();

	let video_stream = input
		.streams()
		.best(ffmpeg::media::Type::Video)
		.ok_or_else(|| Error::CannotFindVideoStream(video_file.as_ref().to_path_buf()))?;

	let video_stream_parameters = video_stream.parameters();
	let (width, height) = unsafe {
		(
			(*video_stream_parameters.as_ptr()).width,
			(*video_stream_parameters.as_ptr()).height,
		)
	};
	#[allow(clippy::cast_sign_loss)]
	let resolution = Resolution::new(width as u32, height as u32);

	let video_codec = unsafe {
		let av_codec_id = ffmpeg::ffi::avcodec_descriptor_get((*video_stream_parameters.as_ptr()).codec_id);
		if av_codec_id.is_null() {
			None
		} else {
			match (*av_codec_id).name {
				name_ptr if name_ptr.is_null() => None,
				name_ptr => Some(String::from_utf8_lossy(CStr::from_ptr(name_ptr).to_bytes()).to_string()),
			}
		}
	};

	let frame_rate = video_stream.rate();

	let Ok(frame_count) = u64::try_from(video_stream.frames()) else {
		unreachable!()
	};

	Ok(Result {
		frame_count,
		frame_rate,
		has_audio,
		resolution,
		video_codec,
	})
}
