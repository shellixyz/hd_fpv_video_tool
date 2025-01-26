use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitStatus;

use derive_more::From;
use ffmpeg_next::Rational;
use itertools::Itertools;
use std::io::Error as IOError;
use thiserror::Error;

pub use self::probe::probe;
use crate::cli::font_options::OSDFontDirError;
use crate::cli::start_end_args::StartEndArgs;
use crate::cli::transcode_video_args::OutputVideoFileError;
use crate::ffmpeg;
use crate::file::TouchError;
use crate::osd::file::{GenericReader, ReadError as OSDFileReadError, UnrecognizedOSDFile};
use crate::osd::overlay::SendFramesToFFMpegError;
use crate::osd::tile_indices::UnknownOSDItem;
use crate::process::Command as ProcessCommand;
use crate::{
	cli::transcode_video_args::TranscodeVideoOSDArgs,
	prelude::{Scaling, TranscodeVideoArgs},
};
use crate::{osd::overlay::scaling::ScalingArgsError, prelude::*};

pub mod coordinates;
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

pub async fn cut<P: AsRef<Path>, Q: AsRef<Path>>(
	input_video_file: P,
	output_video_file: &Option<Q>,
	overwrite: bool,
	start_end: &StartEndArgs,
) -> Result<(), CutVideoError> {
	let input_video_file = input_video_file.as_ref();

	if !input_video_file.exists() {
		return Err(CutVideoError::InputVideoFileDoesNotExist);
	}

	let output_video_file = match output_video_file {
		Some(output_video_file) => {
			let output_video_file = output_video_file.as_ref();
			if input_video_file == output_video_file {
				return Err(CutVideoError::InputAndOutputFileIsTheSame);
			}
			let (input_file_extension, output_file_extension) =
				(input_video_file.extension(), output_video_file.extension());
			if input_file_extension.is_none() != output_file_extension.is_none()
				|| matches!((input_file_extension, output_file_extension), (Some(i), Some(o)) if i.to_ascii_lowercase() != o.to_ascii_lowercase())
			{
				return Err(CutVideoError::OutputHasADifferentExtensionThanInput);
			}
			output_video_file.to_path_buf()
		},
		None => {
			let mut output_file_stem =
				Path::new(input_video_file.file_stem().ok_or(CutVideoError::InputHasNoFileName)?)
					.as_os_str()
					.to_os_string();
			output_file_stem.push("_cut");
			let input_file_extension = input_video_file.extension().ok_or(CutVideoError::InputHasNoExtension)?;
			input_video_file
				.with_file_name(output_file_stem)
				.with_extension(input_file_extension)
		},
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
		&start_end.start(),
		&start_end.end(),
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

	ffmpeg_command
		.build()
		.unwrap()
		.spawn_with_progress(frame_count)?
		.wait()
		.await?;

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
	pub fn sync(&self) -> bool {
		use AudioFixType::*;
		matches!(self, Sync | SyncAndVolume)
	}

	pub fn volume(&self) -> bool {
		use AudioFixType::*;
		matches!(self, Volume | SyncAndVolume)
	}

	fn ffmpeg_audio_filter_string(&self) -> String {
		use AudioFixType::*;
		match self {
			Sync => "atempo=1.001480".to_owned(),
			Volume => "volume=20".to_owned(),
			SyncAndVolume => [Sync.ffmpeg_audio_filter_string(), Volume.ffmpeg_audio_filter_string()].join(","),
		}
	}
}

pub async fn fix_dji_air_unit_audio<P: AsRef<Path>, Q: AsRef<Path>>(
	input_video_file: P,
	output_video_file: &Option<Q>,
	overwrite: bool,
	fix_type: AudioFixType,
) -> Result<(), FixVideoFileAudioError> {
	let input_video_file = input_video_file.as_ref();

	if !input_video_file.exists() {
		return Err(FixVideoFileAudioError::InputVideoFileDoesNotExist);
	}

	let output_video_file = match output_video_file {
		Some(output_video_file) => {
			let output_video_file = output_video_file.as_ref();
			if input_video_file == output_video_file {
				return Err(FixVideoFileAudioError::InputAndOutputFileIsTheSame);
			}
			let (input_file_extension, output_file_extension) =
				(input_video_file.extension(), output_video_file.extension());
			if input_file_extension.is_none() != output_file_extension.is_none()
				|| matches!((input_file_extension, output_file_extension), (Some(i), Some(o)) if i.to_ascii_lowercase() != o.to_ascii_lowercase())
			{
				return Err(FixVideoFileAudioError::OutputHasADifferentExtensionThanInput);
			}
			output_video_file.to_path_buf()
		},
		None => {
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
		},
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

	ffmpeg_command
		.build()
		.unwrap()
		.spawn_with_progress(video_info.frame_count())?
		.wait()
		.await?;

	log::info!("video file's audio stream fixed successfully");
	Ok(())
}

fn frame_count_for_interval(
	total_frames: u64,
	frame_rate: Rational,
	start: &Option<Timestamp>,
	end: &Option<Timestamp>,
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
		use SendFramesToFFMpegError::*;
		match error {
			PipeError(error) => Self::FailedSendingOSDFramesToFFMpeg(error),
			UnknownOSDItem(error) => Self::UnknownOSDItem(error),
			FFMpegExitedWithError(error) => Self::FFMpegExitedWithError(error),
		}
	}
}

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

	let video_info = probe(args.input_video_file())?;
	let frame_count = frame_count_for_interval(
		video_info.frame_count(),
		video_info.frame_rate(),
		&args.start_end().start(),
		&args.start_end().end(),
	);

	let mut ffmpeg_command = ffmpeg::CommandBuilder::default();

	ffmpeg_command
		.add_input_file_slice(
			args.input_video_file(),
			args.start_end().start(),
			args.start_end().end(),
		)
		.set_output_video_settings(
			Some(args.video_encoder()),
			Some(args.video_bitrate()),
			Some(args.video_crf()),
		)
		.set_output_file(output_video_file.clone())
		.set_overwrite_output_file(true);

	if !args.remove_video_defects().is_empty() {
		let defect_filter = args
			.remove_video_defects()
			.iter()
			.map(|region| format!("delogo={}", region.to_ffmpeg_filter_string()))
			.join(";");
		let complex_filter = if let Some(resolution) = args.video_resolution() {
			let resolution_dimensions = resolution.dimensions();
			format!(
				"[0]{}[s1];[s1]scale={}x{}:flags=lanczos[vo]",
				defect_filter,
				resolution_dimensions.width(),
				resolution_dimensions.height()
			)
		} else {
			format!("[0]{}[vo]", defect_filter)
		};
		ffmpeg_command.add_complex_filter(&complex_filter).add_mapping("[vo]");
		if video_info.has_audio() {
			ffmpeg_command.add_mapping("0:a");
		}
	} else if let Some(resolution) = args.video_resolution() {
		let resolution_dimensions = resolution.dimensions();
		ffmpeg_command.add_video_filter(&format!(
			"scale={}x{}:flags=lanczos",
			resolution_dimensions.width(),
			resolution_dimensions.height()
		));
	}

	if let Some(video_audio_fix) = args.video_audio_fix() {
		if video_info.has_audio() {
			ffmpeg_command
				.add_audio_filter(&video_audio_fix.ffmpeg_audio_filter_string())
				.set_output_audio_settings(Some(args.audio_encoder()), Some(args.audio_bitrate()));
		}
	}

	ffmpeg_command
		.build()
		.unwrap()
		.spawn_with_progress(frame_count)?
		.wait()
		.await?;

	log::info!("{frame_count} frames transcoded successfully");
	Ok(output_video_file)
}

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

	if video_info.frame_rate().numerator() != 60 || video_info.frame_rate().denominator() != 1 {
		return Err(TranscodeVideoError::CanOnlyBurnOSDOn60FPSVideo(
			video_info.frame_rate().numerator() as f64 / video_info.frame_rate().denominator() as f64,
		));
	}

	let osd_scaling = Scaling::try_from_osd_args(osd_args.osd_scaling_args(), video_info.resolution())?;
	let mut osd_file = osd::file::open(osd_file_path)?;
	let osd_font_dir = FontDir::new(osd_args.osd_font_options().osd_font_dir()?);
	let osd_frames_generator = OverlayGenerator::new(
		osd_file.frames()?,
		osd_file.font_variant(),
		&osd_font_dir,
		&osd_args.osd_font_options().osd_font_ident(),
		osd_scaling,
		osd_args.osd_hide_regions(),
		osd_args.osd_hide_items(),
	)?;

	let frame_count = frame_count_for_interval(
		video_info.frame_count(),
		video_info.frame_rate(),
		&args.start_end().start(),
		&args.start_end().end(),
	);
	log::debug!(
		"frame count: video={}, transcode={}",
		video_info.frame_count(),
		frame_count
	);

	let first_frame_index = args
		.start_end()
		.start()
		.map(|tstamp| tstamp.frame_count(video_info.frame_rate()) as u32)
		.unwrap_or(0);
	let last_frame_index = args
		.start_end()
		.end()
		.map(|end| end.frame_count(video_info.frame_rate()) as u32)
		.unwrap_or(frame_count as u32);
	let osd_overlay_resolution = osd_frames_generator.frame_dimensions();
	let osd_frames_iter =
		osd_frames_generator.iter_advanced(first_frame_index, Some(last_frame_index), osd_frame_shift);

	let mut ffmpeg_command = ffmpeg::CommandBuilder::default();

	let complex_filter = if args.remove_video_defects().is_empty() {
		"[0][1]overlay=eof_action=repeat:x=(W-w)/2:y=(H-h)/2[vo]".to_owned()
	} else {
		let defect_filter = args
			.remove_video_defects()
			.iter()
			.map(|region| format!("delogo={}", region.to_ffmpeg_filter_string()))
			.join(";");
		format!(
			"[0]{}[s1];[s1][1]overlay=eof_action=repeat:x=(W-w)/2:y=(H-h)/2[vo]",
			defect_filter
		)
	};

	ffmpeg_command
		.add_input_file_slice(
			args.input_video_file(),
			args.start_end().start(),
			args.start_end().end(),
		)
		.add_stdin_input(osd_overlay_resolution, 60)
		.unwrap()
		.add_complex_filter(&complex_filter)
		.add_mapping("[vo]")
		.set_output_video_settings(
			Some(args.video_encoder()),
			Some(args.video_bitrate()),
			Some(args.video_crf()),
		)
		.set_output_file(output_video_file)
		.set_overwrite_output_file(true);

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

	let ffmpeg_process = ffmpeg_command.build().unwrap().spawn_with_progress(frame_count)?;

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

pub fn play_with_osd<P: AsRef<Path>, Q: AsRef<Path>>(
	video_file: P,
	osd_video_file: &Option<Q>,
) -> Result<(), PlayWithOSDError> {
	let video_file = video_file.as_ref();

	let osd_video_file = match osd_video_file {
		Some(osd_video_file) => osd_video_file.as_ref().to_path_buf(),
		None => {
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
		},
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
