
use std::{path::{Path, PathBuf}, process::{Command, Stdio, ExitStatus, Child}, io::Read};

use clap::Args;
use derive_more::From;
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use thiserror::Error;
use std::io::Error as IOError;
use ffmpeg_next as ffmpeg;
use ffmpeg::{format, media};
use lazy_static::lazy_static;

use crate::osd::overlay::Generator as OSDOverlayFramesGenerator;


#[derive(Debug, Error, From)]
pub enum TranscodeVideoError {
    #[error("failed to get input video details")]
    FailedToGetInputVideoDetails(VideoProbingError),
    #[error("input video file does not exist")]
    InputVideoFileDoesNotExist,
    #[error("output video file exists")]
    OutputVideoFileExists,
    #[error("failed spawning ffmpeg process: {0}")]
    #[from(ignore)]
    FailedSpawningFFMpegProcess(IOError),
    #[error("ffmpeg process exited with error: {0}")]
    FFMpegExitedWithError(i32),
}

#[derive(Args)]
pub struct TranscodeArgs {
    #[clap(short, long, value_parser, default_value = "libx265")]
    encoder: String,

    #[clap(short, long, value_parser, default_value = "25M")]
    bitrate: String,

    #[clap(short, long, value_parser, default_value_t = 30)]
    crf: u8,

    #[clap(long, value_parser)]
    start: Option<String>,

    #[clap(long, value_parser)]
    end: Option<String>,
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

fn video_total_frames<P: AsRef<Path>>(video_file: P) -> Result<u64, VideoProbingError> {
    ffmpeg::init().unwrap();
    ffmpeg::log::set_level(ffmpeg::log::Level::Quiet);
    let ictx = format::input(&video_file).map_err(|_| VideoProbingError::new(&video_file, "failed to open video file"))?;
    let video_stream = ictx.streams().best(media::Type::Video).ok_or_else(|| VideoProbingError::new(&video_file, "cannot find video stream"))?;
    let frames = u64::try_from(video_stream.frames()).map_err(|_| VideoProbingError::new(&video_file, "failed to get frame count"))?;
    Ok(frames)
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

pub fn transcode_video<P: AsRef<Path>, Q: AsRef<Path>>(input_video_file: P, output_video_file: Q, args: &TranscodeArgs) -> Result<(), TranscodeVideoError> {
    if ! input_video_file.as_ref().exists() { return Err(TranscodeVideoError::InputVideoFileDoesNotExist); }
    if output_video_file.as_ref().exists() { return Err(TranscodeVideoError::OutputVideoFileExists); }
    log::info!("transcoding video: {} -> {}", input_video_file.as_ref().to_string_lossy(), output_video_file.as_ref().to_string_lossy());

    let frame_count = video_total_frames(&input_video_file)?;

    let mut ffmpeg_command = Command::new("ffmpeg");
    let ffmpeg_command_with_args = &mut ffmpeg_command;

    if let Some(start) = &args.start {
        ffmpeg_command_with_args.args(["-ss", start.as_str()]);
    }

    if let Some(end) = &args.start {
        ffmpeg_command_with_args.args(["-to", end.as_str()]);
    }

    ffmpeg_command_with_args
        .arg("-i").arg(input_video_file.as_ref().as_os_str())
        .args([
            "-c:a", "copy",
            "-c:v", args.encoder.as_str(),
            "-crf", args.crf.to_string().as_str(),
            "-b:v", args.bitrate.as_str(),
            "-y"
        ])
        .arg(output_video_file.as_ref().as_os_str());

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

pub fn transcode_video_burn_osd<P: AsRef<Path>, Q: AsRef<Path>>(input_video_file: P, output_video_file: Q, args: &TranscodeArgs, osd_frames_generator: OSDOverlayFramesGenerator) -> anyhow::Result<()> {

    Ok(())
}