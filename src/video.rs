use std::{
	ffi::OsString,
	io::Error as IOError,
	path::{Path, PathBuf},
	process::ExitStatus,
};

use derive_more::From;
use ffmpeg_next::Rational;
pub use hw_accel::HwAcceleratedEncoding;
use itertools::Itertools;
use thiserror::Error;

pub use self::{codec::Codec, probe::probe};
use crate::{
	AsBool,
	cli::{
		font_options::OSDFontDirError,
		start_end_args::CutVideoStartEndArgs,
		transcode_video_args::{OutputVideoFileError, TranscodeVideoOSDArgs, TranscodeVideoProfileError},
	},
	ffmpeg::{self, VideoQuality},
	file::TouchError,
	osd::{
		file::{GenericReader, ReadError as OSDFileReadError, UnrecognizedOSDFile},
		overlay::{SendFramesToFFMpegError, scaling::ScalingArgsError},
		tile_indices::UnknownOSDItem,
	},
	prelude::{Scaling, TranscodeVideoArgs, *},
	process::Command as ProcessCommand,
};

pub mod codec;
pub mod coordinates;
pub mod hw_accel;
pub mod probe;
pub mod region;
pub mod resolution;
pub mod timestamp;

pub use coordinates::{
	Coordinate, Coordinates, FormatError as CoordinatesFormatError, SignedCoordinate, SignedCoordinates,
};
pub use region::Region;
pub use resolution::Resolution;
pub(crate) use resolution::margins;
pub use timestamp::Timestamp;

pub type Dimension = u16;
pub type Dimensions = GenericDimensions<Dimension>;
pub type FrameIndex = u32;

const DJI_AUDIO_FIX_ATEMPO: f64 = 1.001_480;
const DJI_AUDIO_FIX_VOLUME: isize = 20;

#[derive(Debug, Error, From)]
pub enum CutVideoError {
	#[error("failed to get input video details")]
	FailedToGetInputVideoDetails(VideoProbingError),
	#[error("input video file does not exist")]
	InputVideoFileDoesNotExist,
	#[error("output video file exists")]
	OutputVideoFileExists,
	#[error("input file and output file are the same file")]
	InputAndOutputFileIsTheSame,
	#[error("input has no file name")]
	InputHasNoFileName,
	#[error("input has no extension")]
	InputHasNoExtension,
	#[error("output file has a different extension than input")]
	OutputHasADifferentExtensionThanInput,
	#[error(transparent)]
	FailedSpawningFFMpegProcess(ffmpeg::SpawnError),
	#[error(transparent)]
	FFMpegExitedWithError(ffmpeg::ProcessError),
	#[error(transparent)]
	WriteToFileError(TouchError),
}

/// Cut a video file between the given start and end timestamps.
///
/// # Errors
/// Returns an error if the cutting process fails.
///
/// # Panics
/// Panics if the ffmpeg command cannot be built.
pub async fn cut<P: AsRef<Path>, Q: AsRef<Path>>(
	input_video_file: P,
	output_video_file: &Option<Q>,
	overwrite: bool,
	start_end: &CutVideoStartEndArgs,
	ffmpeg_priority: Option<i32>,
) -> Result<(), CutVideoError> {
	let input_video_file = input_video_file.as_ref();

	if !input_video_file.exists() {
		return Err(CutVideoError::InputVideoFileDoesNotExist);
	}

	let output_video_file = if let Some(output_video_file) = output_video_file {
		let output_video_file = output_video_file.as_ref();
		if input_video_file == output_video_file {
			return Err(CutVideoError::InputAndOutputFileIsTheSame);
		}
		let (input_file_extension, output_file_extension) =
			(input_video_file.extension(), output_video_file.extension());
		if input_file_extension.is_none() != output_file_extension.is_none()
			|| matches!((input_file_extension, output_file_extension), (Some(i), Some(o)) if !i.eq_ignore_ascii_case(o))
		{
			return Err(CutVideoError::OutputHasADifferentExtensionThanInput);
		}
		output_video_file.to_path_buf()
	} else {
		let mut output_file_stem = Path::new(input_video_file.file_stem().ok_or(CutVideoError::InputHasNoFileName)?)
			.as_os_str()
			.to_os_string();
		output_file_stem.push("_cut");
		let input_file_extension = input_video_file.extension().ok_or(CutVideoError::InputHasNoExtension)?;
		input_video_file
			.with_file_name(output_file_stem)
			.with_extension(input_file_extension)
	};

	if !overwrite && output_video_file.exists() {
		return Err(CutVideoError::OutputVideoFileExists);
	}

	file::touch(&output_video_file)?;

	log::info!(
		"cutting video: {} -> {}",
		input_video_file.to_string_lossy(),
		output_video_file.to_string_lossy()
	);

	let video_info = probe(input_video_file)?;
	let frame_count = frame_count_for_interval(
		video_info.frame_count(),
		video_info.frame_rate(),
		start_end.start().as_ref(),
		start_end.end().as_ref(),
	);

	let mut ffmpeg_command = ffmpeg::CommandBuilder::default();

	ffmpeg_command
		.add_input_file_slice(input_video_file, start_end.start(), start_end.end())
		.set_output_video_codec(Some("copy"))
		.set_output_file(output_video_file)
		.set_overwrite_output_file(true);

	if video_info.has_audio() {
		ffmpeg_command.set_output_audio_codec(Some("copy"));
	}

	let spawn_options = ffmpeg::SpawnOptions::default()
		.with_progress(frame_count)
		.with_priority(ffmpeg_priority);
	ffmpeg_command.build().unwrap().spawn(&spawn_options)?.wait().await?;

	log::info!("video file cut successfully");
	Ok(())
}

#[derive(Debug, Error, From)]
pub enum FixVideoFileAudioError {
	#[error("failed to get input video details")]
	FailedToGetInputVideoDetails(VideoProbingError),
	#[error("input video file does not exist")]
	InputVideoFileDoesNotExist,
	#[error("output video file exists")]
	OutputVideoFileExists,
	#[error("input file and output file are the same file")]
	InputAndOutputFileIsTheSame,
	#[error("input has no file name")]
	InputHasNoFileName,
	#[error("input has no extension")]
	InputHasNoExtension,
	#[error("output file has a different extension than input")]
	OutputHasADifferentExtensionThanInput,
	#[error(transparent)]
	FailedSpawningFFMpegProcess(ffmpeg::SpawnError),
	#[error(transparent)]
	FFMpegExitedWithError(ffmpeg::ProcessError),
	#[error("the input video file does not have an audio stream")]
	InputVideoDoesNotHaveAnAudioStream,
	#[error(transparent)]
	WriteToFileError(TouchError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioFixType {
	Sync,
	Volume,
	SyncAndVolume,
}

impl AudioFixType {
	#[must_use]
	pub fn sync(&self) -> bool {
		use AudioFixType::{Sync, SyncAndVolume};
		matches!(self, Sync | SyncAndVolume)
	}

	#[must_use]
	pub fn volume(&self) -> bool {
		use AudioFixType::{SyncAndVolume, Volume};
		matches!(self, Volume | SyncAndVolume)
	}

	fn ffmpeg_audio_filter_string(&self) -> String {
		use AudioFixType::{Sync, SyncAndVolume, Volume};
		match self {
			Sync => format!("atempo={DJI_AUDIO_FIX_ATEMPO}"),
			Volume => format!("volume={DJI_AUDIO_FIX_VOLUME}"),
			SyncAndVolume => [Sync.ffmpeg_audio_filter_string(), Volume.ffmpeg_audio_filter_string()].join(","),
		}
	}
}

/// Fix the audio stream of a DJI Air Unit video file.
///
/// # Errors
/// Returns an error if the process fails.
///
/// # Panics
/// Panics if the ffmpeg command cannot be built.
pub async fn fix_dji_air_unit_audio<P: AsRef<Path>, Q: AsRef<Path>>(
	input_video_file: P,
	output_video_file: Option<&Q>,
	overwrite: bool,
	fix_type: AudioFixType,
	ffmpeg_priority: Option<i32>,
) -> Result<(), FixVideoFileAudioError> {
	let input_video_file = input_video_file.as_ref();

	if !input_video_file.exists() {
		return Err(FixVideoFileAudioError::InputVideoFileDoesNotExist);
	}

	let output_video_file = if let Some(output_video_file) = output_video_file {
		let output_video_file = output_video_file.as_ref();
		if input_video_file == output_video_file {
			return Err(FixVideoFileAudioError::InputAndOutputFileIsTheSame);
		}
		let (input_file_extension, output_file_extension) =
			(input_video_file.extension(), output_video_file.extension());
		if input_file_extension.is_none() != output_file_extension.is_none()
			|| matches!((input_file_extension, output_file_extension), (Some(i), Some(o)) if !i.eq_ignore_ascii_case(o))
		{
			return Err(FixVideoFileAudioError::OutputHasADifferentExtensionThanInput);
		}
		output_video_file.to_path_buf()
	} else {
		let mut output_file_stem = Path::new(
			input_video_file
				.file_stem()
				.ok_or(FixVideoFileAudioError::InputHasNoFileName)?,
		)
		.as_os_str()
		.to_os_string();
		output_file_stem.push("_fixed_audio");
		let input_file_extension = input_video_file
			.extension()
			.ok_or(FixVideoFileAudioError::InputHasNoExtension)?;
		input_video_file
			.with_file_name(output_file_stem)
			.with_extension(input_file_extension)
	};

	if !overwrite && output_video_file.exists() {
		return Err(FixVideoFileAudioError::OutputVideoFileExists);
	}

	file::touch(&output_video_file)?;

	log::info!(
		"fixing video file audio: {} -> {}",
		input_video_file.to_string_lossy(),
		output_video_file.to_string_lossy()
	);

	let video_info = probe(input_video_file)?;

	if !video_info.has_audio() {
		return Err(FixVideoFileAudioError::InputVideoDoesNotHaveAnAudioStream);
	}

	let mut ffmpeg_command = ffmpeg::CommandBuilder::default();

	ffmpeg_command
		.add_input_file(input_video_file)
		.add_audio_filter(&fix_type.ffmpeg_audio_filter_string())
		.set_output_video_codec(Some("copy"))
		.set_output_audio_settings(Some("aac"), Some("93k"))
		.set_output_file(output_video_file)
		.set_overwrite_output_file(true);

	let spawn_options = ffmpeg::SpawnOptions::default()
		.with_progress(video_info.frame_count())
		.with_priority(ffmpeg_priority);
	ffmpeg_command.build().unwrap().spawn(&spawn_options)?.wait().await?;

	log::info!("video file's audio stream fixed successfully");
	Ok(())
}

fn frame_count_for_interval(
	total_frames: u64,
	frame_rate: Rational,
	start: Option<&Timestamp>,
	end: Option<&Timestamp>,
) -> u64 {
	match (start, end) {
		(None, None) => total_frames,
		(None, Some(end)) => Timestamp::interval_frames(&Timestamp::default(), end, frame_rate),
		(Some(start), None) => total_frames - Timestamp::interval_frames(&Timestamp::default(), start, frame_rate),
		(Some(start), Some(end)) => Timestamp::interval_frames(start, end, frame_rate),
	}
}

#[derive(Debug, Error, From)]
pub enum TranscodeVideoError {
	#[error(transparent)]
	OSDFontDirError(OSDFontDirError),
	#[error(transparent)]
	OutputVideoFileError(OutputVideoFileError),
	#[error(transparent)]
	TranscodeVideoProfileError(TranscodeVideoProfileError),
	#[error(transparent)]
	UnrecognizedOSDFile(UnrecognizedOSDFile),
	#[error(transparent)]
	ScalingArgsError(ScalingArgsError),
	#[error(transparent)]
	DrawFrameOverlayError(DrawFrameOverlayError),
	#[error("failed to get input video details")]
	FailedToGetInputVideoDetails(VideoProbingError),
	#[error("it is only possible to burn the OSD on 60FPS videos, given video is {0:.1}FPS")]
	CanOnlyBurnOSDOn60FPSVideo(f64),
	#[error("requested to fix audio but input has no audio stream")]
	RequestedAudioFixingButInputHasNoAudio,
	#[error("input video file does not exist")]
	InputVideoFileDoesNotExist,
	#[error("output video file exists")]
	OutputVideoFileExists,
	#[error("input file and output file are the same file")]
	InputAndOutputFileIsTheSame,
	#[error("incompatible arguments: {0}")]
	IncompatibleArguments(String),
	#[error("OSD file read error: {0}")]
	OSDFileReadError(OSDFileReadError),
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

impl From<SendFramesToFFMpegError> for TranscodeVideoError {
	fn from(error: SendFramesToFFMpegError) -> Self {
		match error {
			SendFramesToFFMpegError::PipeError(error) => Self::FailedSendingOSDFramesToFFMpeg(error),
			SendFramesToFFMpegError::UnknownOSDItem(error) => Self::UnknownOSDItem(error),
			SendFramesToFFMpegError::FFMpegExitedWithError(error) => Self::FFMpegExitedWithError(error),
		}
	}
}

fn remove_video_defects_regions_are_inside_video_frame(regions: &[Region], video_resolution: Resolution) -> bool {
	for rvd_arg in regions {
		let x_range = 1..(i32::try_from(video_resolution.width()).unwrap() - i32::from(rvd_arg.dimensions().width()));
		let y_range = 1..(i32::try_from(video_resolution.height()).unwrap() - i32::from(rvd_arg.dimensions().height()));
		if !x_range.contains(&i32::from(rvd_arg.top_left_corner().x()))
			|| !y_range.contains(&i32::from(rvd_arg.top_left_corner().y()))
		{
			return false;
		}
	}
	true
}

fn transcode_video_filter_parts(
	args: &TranscodeVideoArgs,
	video_info: &video::probe::Result,
	hw_acceleration: HwAcceleratedEncoding,
) -> Result<Vec<String>, TranscodeVideoError> {
	let mut video_filter_parts = Vec::new();

	if !args.remove_video_defects().is_empty() {
		if !remove_video_defects_regions_are_inside_video_frame(args.remove_video_defects(), video_info.resolution()) {
			return Err(TranscodeVideoError::IncompatibleArguments(
				"cannot remove video defects that are outside the video frame".to_owned(),
			));
		}
		let mut defect_filters = args
			.remove_video_defects()
			.iter()
			.map(|region| format!("delogo={}", region.to_ffmpeg_filter_string()))
			.collect_vec();
		video_filter_parts.append(&mut defect_filters);
	}

	if hw_acceleration.is_no() {
		if let Some(resolution) = args.video_resolution() {
			let resolution_dimensions = resolution.dimensions();
			video_filter_parts.push(format!(
				"scale={}x{}:flags=lanczos",
				resolution_dimensions.width(),
				resolution_dimensions.height()
			));
		}
	}

	if hw_acceleration.is_yes() {
		video_filter_parts.push("format=nv12,hwupload".to_string());
		if let Some(resolution) = args.video_resolution() {
			let resolution_dimensions = resolution.dimensions();
			video_filter_parts.push(format!(
				"scale_vaapi={}:{}",
				resolution_dimensions.width(),
				resolution_dimensions.height()
			));
		}
	}

	if let Some(speed) = args.speed() {
		if speed <= 0.0 {
			return Err(TranscodeVideoError::IncompatibleArguments(
				"speedup factor must be greater than 0".to_owned(),
			));
		}
		let pts_factor = 1.0 / speed;
		video_filter_parts.push(format!("setpts={pts_factor:.6}*PTS"));
	}

	Ok(video_filter_parts)
}

/// Transcode the video file according to the given arguments.
///
/// # Errors
/// Returns an error if the transcoding process fails.
///
/// # Panics
/// Panics if the ffmpeg command cannot be built.
#[allow(clippy::too_many_lines)]
pub async fn transcode(args: &TranscodeVideoArgs) -> Result<PathBuf, TranscodeVideoError> {
	let output_video_file = args.output_video_file(false)?;
	if !args.input_video_file().exists() {
		return Err(TranscodeVideoError::InputVideoFileDoesNotExist);
	}
	if !args.overwrite() && output_video_file.exists() {
		return Err(TranscodeVideoError::OutputVideoFileExists);
	}
	if *args.input_video_file() == output_video_file {
		return Err(TranscodeVideoError::InputAndOutputFileIsTheSame);
	}
	file::touch(&output_video_file)?;
	if args.start_end().start().is_some() && matches!(args.video_audio_fix(), Some(fix) if fix.sync()) {
		return Err(TranscodeVideoError::IncompatibleArguments(
			"cannot fix video audio sync while not starting at the beginning of the file".to_owned(),
		));
	}

	log::info!(
		"transcoding video: {} -> {}",
		args.input_video_file().to_string_lossy(),
		output_video_file.to_string_lossy()
	);

	let (video_codec, hw_acceleration) = args.video_codec()?;

	log::info!(
		"using codec: {} (hw accel: {})",
		video_codec,
		hw_acceleration.to_string().to_lowercase()
	);

	let video_info = probe(args.input_video_file())?;
	let frame_count = frame_count_for_interval(
		video_info.frame_count(),
		video_info.frame_rate(),
		args.start_end().start().as_ref(),
		args.start_end().end().as_ref(),
	);

	let mut ffmpeg_command = ffmpeg::CommandBuilder::default();

	let video_quality = match args.resolved_video_quality(video_codec)? {
		Some(quality) => match hw_acceleration {
			HwAcceleratedEncoding::No => VideoQuality::ConstantRateFactor(quality),
			HwAcceleratedEncoding::Yes => VideoQuality::GlobalQuality(quality),
		},
		None => video_codec.default_video_quality(&hw_acceleration),
	};
	let video_bitrate = args.resolved_video_bitrate(video_codec)?;

	ffmpeg_command
		.add_input_file_slice(
			args.input_video_file(),
			args.start_end().start(),
			args.start_end().end(),
		)
		.set_output_video_settings(
			Some(video_codec.ffmpeg_string(hw_acceleration.as_bool())),
			Some(&video_bitrate),
			Some(video_quality),
			// Some(VideoQuality::GlobalQuality(22)),
		)
		.set_output_file(output_video_file.clone())
		.set_overwrite_output_file(true);

	if args.add_audio() {
		if video_info.has_audio() {
			log::warn!("ignoring request to add audio stream to output video as input has one");
		} else {
			ffmpeg_command.add_input_filter("lavfi", "anullsrc=channel_layout=stereo:sample_rate=48000");
			ffmpeg_command.add_arg("-shortest");
			ffmpeg_command.set_output_audio_settings(Some(args.audio_encoder()), Some(args.audio_bitrate()));
			ffmpeg_command.add_mapping("1:a");
		}
	}

	if hw_acceleration.is_yes() {
		ffmpeg_command.add_prefix_arg("-hwaccel").add_prefix_arg("vaapi");
	}

	let video_filter_parts = transcode_video_filter_parts(args, &video_info, hw_acceleration)?;
	if !video_filter_parts.is_empty() {
		let video_filter = format!("[0:v]{}[vo]", video_filter_parts.join(","));
		ffmpeg_command.add_complex_filter(&video_filter).add_mapping("[vo]");
	}

	if video_info.has_audio() {
		ffmpeg_command.add_mapping("0:a");
		let mut using_audio_filters = false;
		let mut atempo_filter_value = None;
		if let Some(video_audio_fix) = args.video_audio_fix() {
			if video_audio_fix.volume() {
				ffmpeg_command.add_audio_filter(&AudioFixType::Volume.ffmpeg_audio_filter_string());
				using_audio_filters = true;
			}
			if video_audio_fix.sync() {
				let mut atempo = DJI_AUDIO_FIX_ATEMPO;
				if let Some(speed) = args.speed() {
					atempo *= speed;
				}
				atempo_filter_value = Some(atempo);
			}
		} else if let Some(speed) = args.speed() {
			atempo_filter_value = Some(speed);
		}
		if let Some(atempo_filter_value) = atempo_filter_value {
			// atempo filter only supports between 0.5 and 100.0, so chain multiple filters if needed
			let mut remaining_atempo = atempo_filter_value;
			let mut filters = vec![];
			while !(0.5..=100.0).contains(&remaining_atempo) {
				let next_atempo = remaining_atempo.clamp(0.5, 100.0);
				filters.push(format!("atempo={next_atempo:.6}"));
				remaining_atempo /= next_atempo;
			}
			filters.push(format!("atempo={remaining_atempo:.6}"));
			ffmpeg_command.add_audio_filter(&filters.join(","));
			using_audio_filters = true;
		}
		if using_audio_filters {
			ffmpeg_command.set_output_audio_settings(Some(args.audio_encoder()), Some(args.audio_bitrate()));
		}
	}

	let spawn_options = ffmpeg::SpawnOptions::default()
		.with_progress(frame_count)
		.with_priority(*args.ffmpeg_priority());
	ffmpeg_command.build().unwrap().spawn(&spawn_options)?.wait().await?;

	log::info!("{frame_count} frames transcoded successfully");
	Ok(output_video_file)
}

/// Transcode the video file, burning the OSD onto it according to the given arguments.
///
/// # Errors
/// Returns an error if the transcoding process fails.
///
/// # Panics
/// Panics if the ffmpeg command cannot be built.
#[allow(clippy::too_many_lines)]
pub async fn transcode_burn_osd<P: AsRef<Path>>(
	args: &TranscodeVideoArgs,
	osd_file_path: P,
	osd_args: &TranscodeVideoOSDArgs,
) -> Result<(), TranscodeVideoError> {
	let output_video_file = args.output_video_file(true)?;

	if !args.input_video_file().exists() {
		return Err(TranscodeVideoError::InputVideoFileDoesNotExist);
	}
	if !args.overwrite() && output_video_file.exists() {
		return Err(TranscodeVideoError::OutputVideoFileExists);
	}
	if *args.input_video_file() == output_video_file {
		return Err(TranscodeVideoError::InputAndOutputFileIsTheSame);
	}
	file::touch(&output_video_file)?;
	if args.start_end().start().is_some() && matches!(args.video_audio_fix(), Some(fix) if fix.sync()) {
		return Err(TranscodeVideoError::IncompatibleArguments(
			"cannot fix video audio sync while not starting at the beginning of the file".to_owned(),
		));
	}

	let video_info = probe(args.input_video_file())?;

	let osd_frame_shift = match osd_args.osd_frame_shift() {
		Some(frame_shift) => frame_shift,
		None => {
			if video_info.has_audio() {
				let frame_shift = crate::osd::dji::AU_OSD_FRAME_SHIFT;
				log::info!(
					"input video file contains audio, assuming DJI AU origin, applying {frame_shift} OSD frames shift"
				);
				frame_shift
			} else {
				0
			}
		},
	};

	log::info!(
		"transcoding video: {} -> {}",
		args.input_video_file().to_string_lossy(),
		output_video_file.to_string_lossy()
	);

	let (video_codec, hw_acceleration) = args.video_codec()?;

	log::info!(
		"using codec: {} (hw accel: {})",
		video_codec,
		hw_acceleration.to_string().to_lowercase()
	);

	if video_info.frame_rate().numerator() != 60 || video_info.frame_rate().denominator() != 1 {
		return Err(TranscodeVideoError::CanOnlyBurnOSDOn60FPSVideo(
			f64::from(video_info.frame_rate().numerator()) / f64::from(video_info.frame_rate().denominator()),
		));
	}

	let osd_scaling = Scaling::try_from_osd_args(osd_args.osd_scaling_args(), video_info.resolution())?;
	let mut osd_file = osd::file::open(osd_file_path)?;
	let osd_font_dir = FontDir::new(osd_args.osd_font_options().osd_font_dir()?);
	let osd_frames_generator = OverlayGenerator::new(
		osd_file.frames()?,
		osd_file.font_variant(),
		&osd_font_dir,
		osd_args.osd_font_options().osd_font_ident(),
		osd_scaling,
		osd_args.osd_hide_regions(),
		osd_args.osd_hide_items(),
	)?;

	let frame_count = frame_count_for_interval(
		video_info.frame_count(),
		video_info.frame_rate(),
		args.start_end().start().as_ref(),
		args.start_end().end().as_ref(),
	);
	log::debug!(
		"frame count: video={}, transcode={}",
		video_info.frame_count(),
		frame_count
	);

	#[allow(clippy::cast_possible_truncation)]
	let first_frame_index = args
		.start_end()
		.start()
		.map_or(0, |tstamp| tstamp.frame_count(video_info.frame_rate()) as u32);
	#[allow(clippy::cast_possible_truncation)]
	let last_frame_index = args.start_end().end().map_or(frame_count as u32, |end| {
		end.frame_count(video_info.frame_rate()) as u32
	});
	let osd_overlay_resolution = osd_frames_generator.frame_dimensions();
	let osd_frames_iter =
		osd_frames_generator.iter_advanced(first_frame_index, Some(last_frame_index), osd_frame_shift);

	let mut ffmpeg_command = ffmpeg::CommandBuilder::default();

	let video_quality = match args.resolved_video_quality(video_codec)? {
		Some(quality) => match hw_acceleration {
			HwAcceleratedEncoding::No => VideoQuality::ConstantRateFactor(quality),
			HwAcceleratedEncoding::Yes => VideoQuality::GlobalQuality(quality),
		},
		None => video_codec.default_video_quality(&hw_acceleration),
	};
	let video_bitrate = args.resolved_video_bitrate(video_codec)?;

	let overlay_filter = "[0][1]overlay=eof_action=repeat:x=(W-w)/2:y=(H-h)/2";
	let video_filter_parts = transcode_video_filter_parts(args, &video_info, hw_acceleration)?;
	let video_filter = if video_filter_parts.is_empty() {
		format!("{overlay_filter}[vo]")
	} else {
		format!("{}[s1];[s1]{}[vo]", overlay_filter, video_filter_parts.join(","))
	};

	ffmpeg_command
		.add_input_file_slice(
			args.input_video_file(),
			args.start_end().start(),
			args.start_end().end(),
		)
		.add_stdin_input(osd_overlay_resolution, 60)
		.unwrap()
		.add_complex_filter(&video_filter)
		.add_mapping("[vo]")
		.set_output_video_settings(
			Some(video_codec.ffmpeg_string(hw_acceleration.as_bool())),
			Some(&video_bitrate),
			Some(video_quality),
		)
		.set_output_file(output_video_file)
		.set_overwrite_output_file(true);

	if hw_acceleration.is_yes() {
		ffmpeg_command.add_prefix_arg("-hwaccel").add_prefix_arg("vaapi");
	}

	if args.add_audio() {
		if video_info.has_audio() {
			log::warn!("ignoring request to add audio stream to output video as input has one");
		} else {
			ffmpeg_command.add_input_filter("lavfi", "anullsrc=channel_layout=stereo:sample_rate=48000");
			ffmpeg_command.add_arg("-shortest");
			ffmpeg_command.set_output_audio_settings(Some(args.audio_encoder()), Some(args.audio_bitrate()));
			ffmpeg_command.add_mapping("2:a");
		}
	}

	match (video_info.has_audio(), args.video_audio_fix()) {
		(true, None) => {
			ffmpeg_command.add_mapping("0:a");
		},
		(true, Some(audio_fix_type)) => {
			ffmpeg_command
				.add_mapping_with_audio_filter("0:a", &audio_fix_type.ffmpeg_audio_filter_string())
				.set_output_audio_settings(Some(args.audio_encoder()), Some(args.audio_bitrate()));
		},
		(false, None) => {},
		(false, Some(_)) => return Err(TranscodeVideoError::RequestedAudioFixingButInputHasNoAudio),
	}

	let spawn_options = ffmpeg::SpawnOptions::default()
		.with_progress(frame_count)
		.with_priority(*args.ffmpeg_priority());
	let ffmpeg_process = ffmpeg_command.build().unwrap().spawn(&spawn_options)?;

	osd_frames_iter.send_frames_to_ffmpeg_and_wait(ffmpeg_process).await?;

	log::info!("{frame_count} frames transcoded successfully");
	Ok(())
}

#[derive(Debug, Error)]
pub enum PlayWithOSDError {
	#[error("invalid video file path: {0}")]
	InvalidVideoFilePath(PathBuf),
	#[error("OSD file not found: {0}")]
	OSDVideoFileNotFound(PathBuf),
	#[error(transparent)]
	VideoProbingError(#[from] VideoProbingError),
	#[error("can only use OSD video files encoded with VP8 or VP9")]
	CanOnlyUseVP8OrVP9OSDVideoFiles,
	#[error("failed to start MPV")]
	FailedToStartMPV(IOError),
	#[error("MPV exited with an error: {0}")]
	MPVExitedWithAnError(ExitStatus),
}

/// Play the given video file with an OSD overlay using MPV.
///
/// # Errors
/// - Returns `PlayWithOSDError::InvalidVideoFilePath` if the video file path is invalid.
/// - Returns `PlayWithOSDError::OSDVideoFileNotFound` if the OSD video file is not found.
/// - Returns `PlayWithOSDError::CanOnlyUseVP8OrVP9OSDVideoFiles` if the OSD video file is not
///   encoded with VP8 or VP9.
/// - Returns `PlayWithOSDError::FailedToStartMPV` if MPV fails to start.
/// - Returns `PlayWithOSDError::MPVExitedWithAnError` if MPV exits with an error.
///
/// # Panics
/// Panics if failed to wait for the MPV process.
pub fn play_with_osd<P: AsRef<Path>, Q: AsRef<Path>>(
	video_file: P,
	osd_video_file: &Option<Q>,
) -> Result<(), PlayWithOSDError> {
	let video_file = video_file.as_ref();

	let osd_video_file = if let Some(osd_video_file) = osd_video_file {
		osd_video_file.as_ref().to_path_buf()
	} else {
		let video_file_stem = video_file
			.file_stem()
			.ok_or_else(|| PlayWithOSDError::InvalidVideoFilePath(video_file.to_path_buf()))?;
		let mut osd_video_file_name = video_file_stem.to_os_string();
		osd_video_file_name.push("_osd");
		let osd_video_file = video_file.with_file_name(osd_video_file_name).with_extension("webm");
		if !osd_video_file.exists() {
			return Err(PlayWithOSDError::OSDVideoFileNotFound(osd_video_file));
		}
		osd_video_file
	};

	let probe_result = probe(&osd_video_file)?;
	let osd_video_codec = probe_result
		.video_codec()
		.as_deref()
		.ok_or(PlayWithOSDError::CanOnlyUseVP8OrVP9OSDVideoFiles)?;

	let decode_lib = match osd_video_codec {
		"vp8" => "libvpx",
		"vp9" => "libvpx-vp9",
		_ => return Err(PlayWithOSDError::CanOnlyUseVP8OrVP9OSDVideoFiles),
	};

	let mut external_file_arg = OsString::from("--external-file=");
	external_file_arg.push(osd_video_file.as_os_str());

	let mut mpv_command = ProcessCommand::new("mpv");

	mpv_command
		.arg(format!("--vd={decode_lib}"))
		.arg(external_file_arg)
		.arg(video_file)
		.arg("--lavfi-complex=[vid1][vid2]overlay=(main_w-overlay_w)/2:(main_h-overlay_h)/2[vo]");

	let mut mpv_child_proc = mpv_command.spawn().map_err(PlayWithOSDError::FailedToStartMPV)?;

	match mpv_child_proc.wait().unwrap() {
		exit_result if !exit_result.success() => Err(PlayWithOSDError::MPVExitedWithAnError(exit_result)),
		_ => Ok(()),
	}
}

#[derive(Debug, Error, From)]
pub enum SpliceVideosError {
	#[error("empty input video files list")]
	EmptyInputVideoFilesList,
	#[error("failed to get input video details")]
	FailedToGetInputVideoDetails(VideoProbingError),
	#[error("output video file exists")]
	OutputVideoFileExists,
	#[error("input video do not have the same resolution")]
	IncompatibleResolutions,
	#[error("failed to build ffmpeg command: {0}")]
	FailedBuildingFFMpegCommand(ffmpeg::BuildCommandError),
	#[error(transparent)]
	FailedSpawningFFMpegProcess(ffmpeg::SpawnError),
	#[error(transparent)]
	FFMpegExitedWithError(ffmpeg::ProcessError),
	#[error("missing input video files: {0}")]
	MissingInputVideoFiles(String),
}

/// Splice multiple video files into a single output file.
///
/// # Errors
/// Returns an error if the splicing process fails.
pub async fn splice(
	input_files: &[impl AsRef<Path>],
	output_file: impl AsRef<Path>,
	overwrite: bool,
	ffmpeg_priority: Option<i32>,
) -> Result<(), SpliceVideosError> {
	if input_files.is_empty() {
		return Err(SpliceVideosError::EmptyInputVideoFilesList);
	}
	let missing_input_files = input_files
		.iter()
		.filter(|file| !file.as_ref().exists())
		.collect::<Vec<_>>();
	if !missing_input_files.is_empty() {
		return Err(SpliceVideosError::MissingInputVideoFiles(
			missing_input_files
				.iter()
				.map(|file| file.as_ref().to_string_lossy())
				.join(", "),
		));
	}

	let output_file = output_file.as_ref();
	if !overwrite && output_file.exists() {
		return Err(SpliceVideosError::OutputVideoFileExists);
	}

	log::info!(
		"splicing videos: {} -> {}",
		input_files
			.iter()
			.map(|file| file.as_ref().to_string_lossy())
			.join(", "),
		output_file.to_string_lossy()
	);

	let videos_info = input_files.iter().map(probe).try_collect::<_, Vec<_>, _>()?;

	let Some(first_video_resolution) = videos_info.first().map(probe::Result::resolution) else {
		unreachable!();
	};
	if videos_info
		.iter()
		.any(|info| info.resolution() != first_video_resolution)
	{
		return Err(SpliceVideosError::IncompatibleResolutions);
	}

	let some_file_has_audio = videos_info.iter().any(probe::Result::has_audio);
	let some_file_lacks_audio = videos_info.iter().any(|info| !info.has_audio());
	if some_file_has_audio && some_file_lacks_audio {
		log::warn!("some input files have audio streams while others do not, the result will not have audio");
	}

	let (_temp_list_file_path, ffmpeg_command) =
		ffmpeg::CommandBuilder::concat(None, input_files, output_file, overwrite)?;

	let total_frame_count = videos_info.iter().map(probe::Result::frame_count).sum::<u64>();

	let spawn_options = ffmpeg::SpawnOptions::default()
		.with_progress(total_frame_count)
		.with_priority(ffmpeg_priority);
	ffmpeg_command.spawn(&spawn_options)?.wait().await?;

	log::info!("videos spliced successfully, total {total_frame_count} frames");
	Ok(())
}

#[derive(Debug, Error)]
pub enum AddAudioStreamError {
	#[error("input file does not exist: {0}")]
	InputFileDoesNotExist(PathBuf),
	#[error("output video file exists: {0}")]
	OutputVideoFileExists(PathBuf),
	#[error("input file already has an audio stream")]
	InputFileAlreadyHasAudioStream,
	#[error(transparent)]
	FailedSpawningFFMpegProcess(#[from] ffmpeg::SpawnError),
	#[error(transparent)]
	FFMpegExitedWithError(#[from] ffmpeg::ProcessError),
	#[error("failed to get input video details")]
	FailedToGetInputVideoDetails(#[from] VideoProbingError),
}

/// Add an audio stream to a video file that lacks one.
///
/// # Errors
/// Returns an error if the process fails.
///
/// # Panics
/// Panics if the ffmpeg command cannot be built.
pub async fn add_audio_stream(
	input_file: impl AsRef<Path>,
	output_file: impl AsRef<Path>,
	overwrite: bool,
	audio_encoder: &str,
	audio_bitrate: &str,
	ffmpeg_priority: Option<i32>,
) -> Result<(), AddAudioStreamError> {
	let input_file = input_file.as_ref();
	if !input_file.exists() {
		return Err(AddAudioStreamError::InputFileDoesNotExist(input_file.to_path_buf()));
	}

	let output_file = output_file.as_ref();
	if !overwrite && output_file.exists() {
		return Err(AddAudioStreamError::OutputVideoFileExists(output_file.to_path_buf()));
	}

	log::info!(
		"adding audio stream to video: {} -> {}",
		input_file.to_string_lossy(),
		output_file.to_string_lossy()
	);

	let video_info = probe(input_file)?;
	if video_info.has_audio() {
		return Err(AddAudioStreamError::InputFileAlreadyHasAudioStream);
	}

	let mut ffmpeg_command = ffmpeg::CommandBuilder::default();

	ffmpeg_command
		.add_input_file(input_file)
		.add_input_filter("lavfi", "anullsrc=channel_layout=stereo:sample_rate=48000")
		.add_arg("-shortest")
		.set_output_video_codec(Some("copy"))
		.set_output_audio_settings(Some(audio_encoder), Some(audio_bitrate))
		.set_output_file(output_file)
		.set_overwrite_output_file(true);

	let spawn_options = ffmpeg::SpawnOptions::default()
		.with_progress(video_info.frame_count())
		.with_priority(ffmpeg_priority);
	ffmpeg_command.build().unwrap().spawn(&spawn_options)?.wait().await?;

	log::info!("audio stream added successfully");

	Ok(())
}
