
use std::{path::Path, io::Write};

use derive_more::From;
use thiserror::Error;
use std::io::Error as IOError;
use ffmpeg_next::Rational;

use crate::{prelude::*, osd::overlay::scaling::ScalingArgsError};
use crate::{prelude::{TranscodeVideoArgs, Scaling}, cli::transcode_video_args::TranscodeVideoOSDArgs};
use crate::osd::dji::file::ReadError as OSDFileReadError;
use crate::ffmpeg;

use self::timestamp::Timestamp;

pub mod timestamp;
pub mod resolution;
pub mod utils;
pub mod probe;


pub type FrameIndex = u32;

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
    #[error("failed spawning ffmpeg process: {0}")]
    #[from(ignore)]
    FailedSpawningFFMpegProcess(IOError),
    #[error("ffmpeg process exited with error: {0}")]
    FFMpegExitedWithError(i32),
    #[error("the input video file does not have an audio stream")]
    InputVideoDoesNotHaveAnAudioStream,
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

pub async fn fix_dji_air_unit_video_file_audio<P: AsRef<Path>, Q: AsRef<Path>>(input_video_file: P, output_video_file: &Option<Q>,
        overwrite: bool, fix_type: AudioFixType) -> Result<(), FixVideoFileAudioError> {

    let input_video_file = input_video_file.as_ref();

    if ! input_video_file.exists() { return Err(FixVideoFileAudioError::InputVideoFileDoesNotExist); }

    let output_video_file = match output_video_file {
        Some(output_video_file) => {
            let output_video_file = output_video_file.as_ref();
            if input_video_file == output_video_file { return Err(FixVideoFileAudioError::InputAndOutputFileIsTheSame) }
            if input_video_file.extension() != output_video_file.extension() { return Err(FixVideoFileAudioError::OutputHasADifferentExtensionThanInput) }
            output_video_file.to_path_buf()
        },
        None => {
            let mut output_file_stem = Path::new(input_video_file.file_stem().ok_or(FixVideoFileAudioError::InputHasNoFileName)?).as_os_str().to_os_string();
            output_file_stem.push("_fixed_audio");
            let input_file_extension = input_video_file.extension().ok_or(FixVideoFileAudioError::InputHasNoExtension)?;
            input_video_file.with_file_name(output_file_stem).with_extension(input_file_extension)
        },
    };

    if output_video_file.exists() { return Err(FixVideoFileAudioError::OutputVideoFileExists); }

    log::info!("fixing video file audio: {} -> {}", input_video_file.to_string_lossy(), output_video_file.to_string_lossy());

    let video_info = video_probe(input_video_file)?;

    if ! video_info.has_audio() {
        return Err(FixVideoFileAudioError::InputVideoDoesNotHaveAnAudioStream);
    }

    let mut ffmpeg_command = ffmpeg::CommandBuilder::default();

    ffmpeg_command
        .add_input_file(input_video_file)
        .add_audio_filter(&fix_type.ffmpeg_audio_filter_string())
        .set_output_audio_settings(Some("aac"), Some("93k"))
        .set_output_file(output_video_file)
        .set_overwrite_output_file(overwrite);

    if let Err(error) = ffmpeg_command.build().unwrap().spawn_with_progress(video_info.frame_count()).unwrap().wait().await {
        return Err(FixVideoFileAudioError::FFMpegExitedWithError(error.exit_status().code().unwrap()))
    }

    log::info!("video file's audio stream fixed successfully");
    Ok(())
}

fn frame_count_for_interval(total_frames: u64, frame_rate: Rational, start: &Option<Timestamp>, end: &Option<Timestamp>) -> u64 {
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
    OSDFileOpenError(OSDFileOpenError),
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
    #[error("failed spawning ffmpeg process: {0}")]
    #[from(ignore)]
    FailedSpawningFFMpegProcess(IOError),
    #[error("failed sending OSD images to ffmpeg process: {0}")]
    FailedSendingOSDImagesToFFMpeg(IOError),
    #[error("ffmpeg process exited with error: {0}")]
    FFMpegExitedWithError(i32),
}

pub async fn transcode_video(args: &TranscodeVideoArgs) -> Result<(), TranscodeVideoError> {

    if ! args.input_video_file().exists() { return Err(TranscodeVideoError::InputVideoFileDoesNotExist); }
    if ! args.overwrite() && args.output_video_file().exists() { return Err(TranscodeVideoError::OutputVideoFileExists); }
    if args.input_video_file() == args.output_video_file() { return Err(TranscodeVideoError::InputAndOutputFileIsTheSame) }
    if args.start_end().start().is_some() && matches!(args.video_audio_fix(), Some(fix) if fix.sync()) {
        return Err(TranscodeVideoError::IncompatibleArguments("cannot fix video audio sync while not starting at the beginning of the file".to_owned()));
    }

    log::info!("transcoding video: {} -> {}", args.input_video_file().to_string_lossy(), args.output_video_file().to_string_lossy());

    let video_info = video_probe(args.input_video_file())?;
    let frame_count = frame_count_for_interval(video_info.frame_count(), video_info.frame_rate(), &args.start_end().start(), &args.start_end().end());

    let mut ffmpeg_command = ffmpeg::CommandBuilder::default();

    ffmpeg_command
        .add_input_file_slice(args.input_video_file(), args.start_end().start(), args.start_end().end())
        .set_output_video_settings(Some(args.video_encoder()), Some(args.video_bitrate()), Some(&args.video_crf().to_string()))
        .set_output_file(args.output_video_file())
        .set_overwrite_output_file(args.overwrite());

    if let Some(video_audio_fix) = args.video_audio_fix() {
        if video_info.has_audio() {
            ffmpeg_command
                .add_audio_filter(&video_audio_fix.ffmpeg_audio_filter_string())
                .set_output_audio_settings(Some(args.audio_encoder()), Some(args.audio_bitrate()));
        }
    }

    if let Err(error) = ffmpeg_command.build().unwrap().spawn_with_progress(frame_count).unwrap().wait().await {
        return Err(TranscodeVideoError::FFMpegExitedWithError(error.exit_status().code().unwrap()))
    }

    log::info!("{frame_count} frames transcoded successfully");
    Ok(())
}

pub async fn transcode_video_burn_osd(args: &TranscodeVideoArgs, osd_args: &TranscodeVideoOSDArgs) -> Result<(), TranscodeVideoError> {

    if ! args.input_video_file().exists() { return Err(TranscodeVideoError::InputVideoFileDoesNotExist); }
    if ! args.overwrite() && args.output_video_file().exists() { return Err(TranscodeVideoError::OutputVideoFileExists); }
    if args.input_video_file() == args.output_video_file() { return Err(TranscodeVideoError::InputAndOutputFileIsTheSame) }
    if args.start_end().start().is_some() && matches!(args.video_audio_fix(), Some(fix) if fix.sync()) {
        return Err(TranscodeVideoError::IncompatibleArguments("cannot fix video audio sync while not starting at the beginning of the file".to_owned()));
    }

    log::info!("transcoding video: {} -> {}", args.input_video_file().to_string_lossy(), args.output_video_file().to_string_lossy());

    let video_info = video_probe(args.input_video_file())?;

    if video_info.frame_rate().numerator() != 60 || video_info.frame_rate().denominator() != 1 {
        return Err(TranscodeVideoError::CanOnlyBurnOSDOn60FPSVideo(video_info.frame_rate().numerator() as f64 / video_info.frame_rate().denominator() as f64))
    }

    let osd_scaling = Scaling::try_from_osd_args(osd_args.osd_scaling_args(), video_info.resolution())?;
    let mut osd_file = OSDFileReader::open(osd_args.osd_file().clone().unwrap())?;
    let osd_font_dir = FontDir::new(&osd_args.osd_font_options().osd_font_dir());
    let osd_frames_generator = OverlayGenerator::new(
        osd_file.frames()?,
        &osd_font_dir,
        &osd_args.osd_font_options().osd_font_ident(),
        osd_scaling
    )?;

    let frame_count = frame_count_for_interval(video_info.frame_count(), video_info.frame_rate(), &args.start_end().start(), &args.start_end().end());

    let first_frame_index = args.start_end().start().map(|tstamp| tstamp.frame_count(video_info.frame_rate()) as u32).unwrap_or(0);
    let last_frame_index = match args.start_end().end() {
        Some(end) => frame_count.min(end.frame_count(video_info.frame_rate())) as u32,
        None => frame_count as u32,
    } - 1;
    let osd_overlay_resolution = osd_frames_generator.frame_dimensions();
    let osd_frames_iter = osd_frames_generator.iter_advanced(first_frame_index, Some(last_frame_index), osd_args.osd_frame_shift());

    let mut ffmpeg_command = ffmpeg::CommandBuilder::default();

    ffmpeg_command
        .add_input_file_slice(args.input_video_file(), args.start_end().start(), args.start_end().end())
        .add_stdin_input(osd_overlay_resolution, 60).unwrap()
        .add_complex_filter("[0][1]overlay=eof_action=repeat:x=(W-w)/2:y=(H-h)/2[vo]")
        .add_mapping("[vo]")
        .set_output_video_settings(Some(args.video_encoder()), Some(args.video_bitrate()), Some(&args.video_crf().to_string()))
        .set_output_file(args.output_video_file())
        .set_overwrite_output_file(args.overwrite());

    match (video_info.has_audio(), args.video_audio_fix()) {
        (true, None) => { ffmpeg_command.add_mapping("0:a"); },
        (true, Some(audio_fix_type)) => {
            ffmpeg_command
                .add_mapping_with_audio_filter("0:a", &audio_fix_type.ffmpeg_audio_filter_string())
                .set_output_audio_settings(Some(args.audio_encoder()), Some(args.audio_bitrate()));
            },
        (false, None) => {},
        (false, Some(_)) => return Err(TranscodeVideoError::RequestedAudioFixingButInputHasNoAudio),
    }

    let mut ffmpeg_process = ffmpeg_command.build().unwrap().spawn_with_progress(video_info.frame_count()).unwrap();
    let mut ffmpeg_stdin = ffmpeg_process.take_stdin().unwrap();

    for osd_frame_image in osd_frames_iter {
        ffmpeg_stdin.write_all(osd_frame_image.as_raw()).map_err(TranscodeVideoError::FailedSendingOSDImagesToFFMpeg)?;
    }

    drop(ffmpeg_stdin);

    if let Err(error) = ffmpeg_process.wait().await {
        return Err(TranscodeVideoError::FFMpegExitedWithError(error.exit_status().code().unwrap()))
    }

    log::info!("{frame_count} frames transcoded successfully");
    Ok(())
}