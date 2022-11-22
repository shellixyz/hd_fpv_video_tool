
use std::{path::{Path, PathBuf}, process::{Command, Stdio, ExitStatus, Child}, io::{Read, Write}};

use clap::Args;
use derive_more::From;
use getset::Getters;
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use thiserror::Error;
use std::io::Error as IOError;
use ffmpeg_next as ffmpeg;
use ffmpeg::{format, media, Rational};
use lazy_static::lazy_static;

use crate::osd::overlay::Generator as OSDOverlayFramesGenerator;
use crate::osd::dji::file::ReadError as OSDFileReadError;

use self::timestamp::{Timestamp, TimestampFormatError};

pub mod timestamp;


#[derive(Debug, Error, From)]
pub enum TranscodeVideoError {
    #[error("failed to get input video details")]
    FailedToGetInputVideoDetails(VideoProbingError),
    #[error("input video file does not exist")]
    InputVideoFileDoesNotExist,
    #[error("output video file exists")]
    OutputVideoFileExists,
    #[error("input file and output file are the same file")]
    InputAndOutputFileIsTheSame,
    #[error("incompatible arguments")]
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

#[derive(Args, Getters)]
#[getset(get = "pub")]
pub struct TranscodeArgs {
    /// fix DJI AU audio: fix sync + volume
    #[clap(short, long, value_parser)]
    fix_audio: bool,

    /// fix DJI AU audio volume
    #[clap(short, long, value_parser)]
    fix_audio_volume: bool,

    /// fix DJI AU audio sync
    #[clap(short, long, value_parser)]
    fix_audio_sync: bool,

    /// video encoder to use
    #[clap(short, long, value_parser, default_value = "libx265")]
    encoder: String,

    /// max bitrate
    #[clap(short, long, value_parser, default_value = "25M")]
    bitrate: String,

    /// constant quality setting
    #[clap(short, long, value_parser, default_value_t = 30)]
    crf: u8,

    /// start timestamp
    #[clap(long, value_parser = timestamp_value_parser, value_name = "[HH:]MM:SS")]
    start: Option<Timestamp>,

    /// end timestamp
    #[clap(long, value_parser = timestamp_value_parser, value_name = "[HH:]MM:SS")]
    end: Option<Timestamp>,
}

fn timestamp_value_parser(timestamp_str: &str) -> Result<Timestamp, TimestampFormatError> {
    Timestamp::try_from(timestamp_str)
}

impl TranscodeArgs {
    pub fn video_audio_fix_type(&self) -> AudioFixType {
        use AudioFixType::*;
        match (self.fix_audio, self.fix_audio_sync, self.fix_audio_volume) {
            (true, _, _) | (false, true, true) => SyncAndVolume,
            (false, true, false) => Sync,
            (false, false, true) => Volume,
            (false, false, false) => None,
        }
    }
}

#[derive(Debug, Error)]
#[error("failed to probe video file {file_path}: {error}")]
pub struct VideoProbingError {
    file_path: PathBuf,
    error: String,
}

impl VideoProbingError {
    pub fn new<P: AsRef<Path>>(file_path: P, error: &str) -> Self {
        Self { file_path: file_path.as_ref().to_path_buf(), error: error.to_owned() }
    }
}

fn video_probe<P: AsRef<Path>>(video_file: P) -> Result<(u64, Rational, bool), VideoProbingError> {
    ffmpeg::init().unwrap();
    ffmpeg::log::set_level(ffmpeg::log::Level::Quiet);
    let input = format::input(&video_file).map_err(|_| VideoProbingError::new(&video_file, "failed to open video file"))?;
    let has_audio_stream = input.streams().best(media::Type::Audio).is_some();
    let video_stream = input.streams().best(media::Type::Video).ok_or_else(|| VideoProbingError::new(&video_file, "cannot find video stream"))?;
    let rate = video_stream.rate();
    let frames = u64::try_from(video_stream.frames()).map_err(|_| VideoProbingError::new(&video_file, "failed to get frame count"))?;
    Ok((frames, rate, has_audio_stream))
}

fn monitor_ffmpeg_progress(frame_count: u64, mut ffmpeg_child: Child) -> ExitStatus {
    let mut ffmpeg_stderr = ffmpeg_child.stderr.take().unwrap();
    let mut output_buf = String::new();
    let mut read_buf = [0; 1024];
    let progress_style = ProgressStyle::with_template("{wide_bar} {percent:>3}% [ETA {eta:>3}]").unwrap();
    let progress_bar = ProgressBar::new(frame_count).with_style(progress_style);
    progress_bar.set_position(0);

    let ffmpeg_result = loop {

        // read new data from stderr and push it into output_buf
        let read_count = ffmpeg_stderr.read(&mut read_buf).unwrap();
        output_buf.push_str(String::from_utf8_lossy(&read_buf[0..read_count]).to_string().as_str());

        // try to find a line which is containing progress data
        let lines = output_buf.split_inclusive('\r').collect::<Vec<_>>();
        let progress_frame = lines.iter().find_map(|line| {
            lazy_static! {
                static ref PROGRESS_RE: Regex = Regex::new(r"\Aframe=\s+(\d+)").unwrap();
            }
            let captures = PROGRESS_RE.captures(line)?;
            let frame: u64 = captures.get(1).unwrap().as_str().parse().unwrap();
            Some(frame)
        });

        // update the progress bar since we just got a progress update
        if let Some(progress_frame) = progress_frame {
            progress_bar.set_position(progress_frame);
        }

        // if last line was incomplete put it back into output_buf otherwise just clear output_buf
        if let Some(last_line) = lines.last() {
            match last_line.chars().last() {
                Some('\r') | None => output_buf.clear(),
                Some(_) => output_buf = last_line.to_string(),
            };
        }

        // check if the ffmpeg process exited and if it did break the loop with the exit status
        if let Some(result) = ffmpeg_child.try_wait().unwrap() {
            break result;
        }

    };

    progress_bar.finish_and_clear();

    ffmpeg_result
}

fn frame_count_for_interval(total_frames: u64, frame_rate: Rational, start: &Option<Timestamp>, end: &Option<Timestamp>) -> u64 {
    match (start, end) {
        (None, None) => total_frames,
        (None, Some(end)) => Timestamp::interval_frames(&Timestamp::default(), end, frame_rate),
        (Some(start), None) => total_frames - Timestamp::interval_frames(&Timestamp::default(), start, frame_rate),
        (Some(start), Some(end)) => Timestamp::interval_frames(start, end, frame_rate),
    }
}

pub fn transcode_video<P: AsRef<Path>, Q: AsRef<Path>>(input_video_file: P, output_video_file: Q, args: &TranscodeArgs) -> Result<(), TranscodeVideoError> {

    if ! input_video_file.as_ref().exists() { return Err(TranscodeVideoError::InputVideoFileDoesNotExist); }
    if output_video_file.as_ref().exists() { return Err(TranscodeVideoError::OutputVideoFileExists); }
    if input_video_file.as_ref() == output_video_file.as_ref() { return Err(TranscodeVideoError::InputAndOutputFileIsTheSame) }
    if args.start.is_some() && matches!(args.video_audio_fix_type(), AudioFixType::Sync | AudioFixType::SyncAndVolume) {
        return Err(TranscodeVideoError::IncompatibleArguments("incompatible arguments: cannot fix video audio sync while not starting at the beginning of the file".to_owned()));
    }

    log::info!("transcoding video: {} -> {}", input_video_file.as_ref().to_string_lossy(), output_video_file.as_ref().to_string_lossy());

    let (frame_count, frame_rate, _has_audio_stream) = video_probe(&input_video_file)?;
    let frame_count = frame_count_for_interval(frame_count, frame_rate, &args.start, &args.end);

    let mut ffmpeg_command = Command::new("ffmpeg");
    let ffmpeg_command_with_args = &mut ffmpeg_command;

    if let Some(start) = &args.start {
        ffmpeg_command_with_args.args(["-ss", start.to_ffmpeg_position().as_str()]);
    }

    if let Some(end) = &args.end {
        ffmpeg_command_with_args.args(["-to", end.to_ffmpeg_position().as_str()]);
    }

    // input args
    ffmpeg_command_with_args.arg("-i").arg(input_video_file.as_ref().as_os_str());

    // audio args
    // XXX don't add the audio args if input video has no audio stream
    ffmpeg_command_with_args.args(args.video_audio_fix_type().ffmpeg_audio_args().iter().map(String::as_str).collect::<Vec<_>>());

    // video args
    ffmpeg_command_with_args.args([
        "-c:v", args.encoder.as_str(),
        "-crf", args.crf.to_string().as_str(),
        "-b:v", args.bitrate.as_str(),
    ]);

    // output args
    ffmpeg_command_with_args.arg("-y").arg(output_video_file.as_ref().as_os_str());

    let ffmpeg_child = ffmpeg_command_with_args
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(TranscodeVideoError::FailedSpawningFFMpegProcess)?;

    let ffmpeg_result = monitor_ffmpeg_progress(frame_count, ffmpeg_child);

    if ! ffmpeg_result.success() {
        return Err(TranscodeVideoError::FFMpegExitedWithError(ffmpeg_result.code().unwrap()))
    }

    log::info!("{frame_count} frames transcoded successfully");
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
    #[error("failed spawning ffmpeg process: {0}")]
    #[from(ignore)]
    FailedSpawningFFMpegProcess(IOError),
    #[error("ffmpeg process exited with error: {0}")]
    FFMpegExitedWithError(i32),
    #[error("the input video file does not have an audio stream")]
    InputVideoDoesNotHaveAnAudioStream,
}

#[derive(Debug, Clone)]
pub enum AudioFixType {
    None,
    Sync,
    Volume,
    SyncAndVolume,
}

impl AudioFixType {

    fn ffmpeg_audio_filter_string(&self) -> Option<String> {
        use AudioFixType::*;
        match self {
            None => Option::None,
            Sync => Some("atempo=1.001480".to_owned()),
            Volume => Some("volume=20".to_owned()),
            SyncAndVolume => Some([Sync.ffmpeg_audio_filter_string().unwrap(), Volume.ffmpeg_audio_filter_string().unwrap()].join(",")),
        }
    }

    fn ffmpeg_audio_args(&self) -> Vec<String> {
        use AudioFixType::*;
        match self {
            None => vec!["-c:a".to_owned(), "copy".to_owned()],
            fix_type => vec![
                "-filter:a".to_owned(), fix_type.ffmpeg_audio_filter_string().unwrap(),
                "-c:a".to_owned(), "aac".to_owned(),
                "-b:a".to_owned(), "93k".to_owned(),
            ]
        }
    }

}

pub fn fix_dji_air_unit_video_file_audio<P: AsRef<Path>, Q: AsRef<Path>>(input_video_file: P, output_video_file: &Option<Q>, fix_type: AudioFixType) -> Result<(), FixVideoFileAudioError> {

    if ! input_video_file.as_ref().exists() { return Err(FixVideoFileAudioError::InputVideoFileDoesNotExist); }

    let output_video_file = match output_video_file {
        Some(output_video_file) => {
            if input_video_file.as_ref() == output_video_file.as_ref() { return Err(FixVideoFileAudioError::InputAndOutputFileIsTheSame) }
            if input_video_file.as_ref().extension() != output_video_file.as_ref().extension() { return Err(FixVideoFileAudioError::OutputHasADifferentExtensionThanInput) }
            output_video_file.as_ref().to_path_buf()
        },
        None => {
            let mut output_file_stem = Path::new(input_video_file.as_ref().file_stem().ok_or(FixVideoFileAudioError::InputHasNoFileName)?).as_os_str().to_os_string();
            output_file_stem.push("_fixed_audio");
            let input_file_extension = input_video_file.as_ref().extension().ok_or(FixVideoFileAudioError::InputHasNoExtension)?;
            input_video_file.as_ref().with_file_name(output_file_stem).with_extension(input_file_extension)
        },
    };

    if output_video_file.exists() { return Err(FixVideoFileAudioError::OutputVideoFileExists); }

    log::info!("fixing video file audio: {} -> {}", input_video_file.as_ref().to_string_lossy(), output_video_file.to_string_lossy());

    let (frame_count, _frame_rate, has_audio_stream) = video_probe(&input_video_file)?;

    if ! has_audio_stream {
        return Err(FixVideoFileAudioError::InputVideoDoesNotHaveAnAudioStream);
    }

    let mut ffmpeg_command = Command::new("ffmpeg");
    let ffmpeg_command_with_args = &mut ffmpeg_command;

    ffmpeg_command_with_args
        .arg("-i").arg(input_video_file.as_ref().as_os_str())
        .args(fix_type.ffmpeg_audio_args().iter().map(String::as_str).collect::<Vec<_>>())
        .arg("-y").arg(output_video_file.as_os_str());

    let ffmpeg_child = ffmpeg_command_with_args
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(FixVideoFileAudioError::FailedSpawningFFMpegProcess)?;

    let ffmpeg_result = monitor_ffmpeg_progress(frame_count, ffmpeg_child);

    if ! ffmpeg_result.success() {
        return Err(FixVideoFileAudioError::FFMpegExitedWithError(ffmpeg_result.code().unwrap()))
    }

    log::info!("video file's audio stream fixed successfully");
    Ok(())
}

pub fn transcode_video_burn_osd<P: AsRef<Path>, Q: AsRef<Path>>(input_video_file: P, output_video_file: Q, args: &TranscodeArgs, mut osd_frames_generator: OSDOverlayFramesGenerator, frame_shift: i32) -> Result<(), TranscodeVideoError> {

    if ! input_video_file.as_ref().exists() { return Err(TranscodeVideoError::InputVideoFileDoesNotExist); }
    if output_video_file.as_ref().exists() { return Err(TranscodeVideoError::OutputVideoFileExists); }
    if input_video_file.as_ref() == output_video_file.as_ref() { return Err(TranscodeVideoError::InputAndOutputFileIsTheSame) }
    if args.start.is_some() && matches!(args.video_audio_fix_type(), AudioFixType::Sync | AudioFixType::SyncAndVolume) {
        return Err(TranscodeVideoError::IncompatibleArguments("incompatible arguments: cannot fix video audio sync while not starting at the beginning of the file".to_owned()));
    }

    log::info!("transcoding video: {} -> {}", input_video_file.as_ref().to_string_lossy(), output_video_file.as_ref().to_string_lossy());

    let (frame_count, frame_rate, _has_audio_stream) = video_probe(&input_video_file)?;
    let frame_count = frame_count_for_interval(frame_count, frame_rate, &args.start, &args.end);

    let first_frame_index = args.start.clone().map(|tstamp| tstamp.frame_index(frame_rate) as u32).unwrap_or(0);
    let last_frame_index = match args.end.clone() {
        Some(end_tstamp) => {
            let end_tstamp_frames = end_tstamp.frame_index(frame_rate);
            frame_count.min(end_tstamp_frames) as u32
        }
        None => frame_count as u32,
    } - 1;
    let osd_overlay_resolution = osd_frames_generator.overlay_resolution();
    let osd_frames_iter = osd_frames_generator.into_iter(first_frame_index, Some(last_frame_index), frame_shift)?;

    let mut ffmpeg_command = Command::new("ffmpeg");
    let ffmpeg_command_with_args = &mut ffmpeg_command;

    if let Some(start) = &args.start {
        ffmpeg_command_with_args.args(["-ss", start.to_ffmpeg_position().as_str()]);
    }

    if let Some(end) = &args.end {
        ffmpeg_command_with_args.args(["-to", end.to_ffmpeg_position().as_str()]);
    }

    // video input args
    ffmpeg_command_with_args.arg("-i").arg(input_video_file.as_ref().as_os_str());

    // overlay input args
    ffmpeg_command_with_args.args([
        "-f", "rawvideo",
        "-pix_fmt", "rgba",
        "-video_size", osd_overlay_resolution.to_string().as_str(),
        "-r", "60",
        "-i", "pipe:0",
    ]);

    // XXX don't add the audio args if input video has no audio stream
    // filter args
    ffmpeg_command_with_args
        .args([
            "-filter_complex", "[0][1]overlay=eof_action=repeat:x=(W-w)/2:y=(H-h)/2[vo]",
            "-map", "[vo]",
            "-map", "0:a",
        ])
        .args(args.video_audio_fix_type().ffmpeg_audio_args().iter().map(String::as_str).collect::<Vec<_>>());

    // video args
    ffmpeg_command_with_args.args([
        "-c:v", args.encoder.as_str(),
        "-crf", args.crf.to_string().as_str(),
        "-b:v", args.bitrate.as_str(),
    ]);

    // output args
    ffmpeg_command_with_args.arg("-y").arg(output_video_file.as_ref().as_os_str());

    dbg!(ffmpeg_command_with_args.get_args().map(|x| x.to_string_lossy()).collect::<Vec<_>>().join(" "));

    let mut ffmpeg_child = ffmpeg_command_with_args
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(TranscodeVideoError::FailedSpawningFFMpegProcess)?;

    let mut ffmpeg_stdin = ffmpeg_child.stdin.take().expect("failed to open ffmpeg stdin");

    for osd_frame_image in osd_frames_iter {
        ffmpeg_stdin.write_all(osd_frame_image.as_raw()).map_err(TranscodeVideoError::FailedSendingOSDImagesToFFMpeg)?;
    }

    drop(ffmpeg_stdin);

    let ffmpeg_result = monitor_ffmpeg_progress(frame_count, ffmpeg_child);
    // let ffmpeg_result = ffmpeg_child.wait().unwrap();

    if ! ffmpeg_result.success() {
        return Err(TranscodeVideoError::FFMpegExitedWithError(ffmpeg_result.code().unwrap()))
    }

    log::info!("{frame_count} frames transcoded successfully");
    Ok(())
}