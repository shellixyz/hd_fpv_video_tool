
use std::{process, path::{Path, PathBuf}, ffi::{OsString, OsStr}, fmt::Display, io::{Error as IOError, Read}};

use derive_more::{Deref, DerefMut};
use getset::{Getters, Setters, CopyGetters};
use indicatif::{ProgressStyle, ProgressBar};
use regex::Regex;
use thiserror::Error;
use lazy_static::lazy_static;
use tokio::task::JoinHandle;

use crate::video::{resolution::Resolution, timestamp::Timestamp};

#[derive(Debug, Clone)]
pub enum Input {
    File {
        path: PathBuf,
        start: Option<Timestamp>,
        end: Option<Timestamp>,
    },
    StdinPipedRaw {
        resolution: Resolution,
        frame_rate: u16,
    }
}

impl Input {
    pub fn to_args(&self) -> Vec<OsString> {
        let mut args = vec![];
        match self {
            Input::File { path, start, end } => {
                if let Some(start) = start {
                    args.push("-ss".into());
                    args.push(start.to_ffmpeg_position().into());
                }
                if let Some(end) = end {
                    args.push("-to".into());
                    args.push(end.to_ffmpeg_position().into());
                }
                args.push("-i".into());
                args.push(path.clone().into_os_string());
            },
            Input::StdinPipedRaw { resolution, frame_rate } => {

                args.append(&mut ["-f", "rawvideo", "-pix_fmt", "rgba", "-video_size" ].map(Into::into).into());
                args.push(resolution.to_string().into());
                args.push("-r".into());
                args.push(frame_rate.to_string().into());
                args.append(&mut ["-i", "pipe:0"].map(Into::into).into());
            },
        }
        args
    }
}

#[derive(Debug, Clone)]
pub enum Filter {
    Audio(String),
    Video(String),
    Complex(String),
}

impl Filter {
    pub fn to_args(&self) -> Vec<OsString> {
        let mut args = vec![];
        let (prefix, value) = match self {
            Filter::Audio(value) => ("-filter:a", value),
            Filter::Video(value) => ("-filter:v", value),
            Filter::Complex(value) => ("-filter_complex", value),
        };
        args.push(prefix.into());
        args.push(value.into());
        args
    }
}

#[derive(Debug, Clone, Getters, Setters)]
#[getset(get = "pub", set = "pub(self)")]
pub struct CommonOutputStreamSettings {
    codec: String,
    bitrate: Option<String>,
}

impl Default for CommonOutputStreamSettings {
    fn default() -> Self {
        Self { codec: "copy".to_owned(), bitrate: Default::default() }
    }
}

#[derive(Debug, Clone, Deref, DerefMut, Default)]
pub struct AudioOutputSettings(CommonOutputStreamSettings);

impl AudioOutputSettings {
    pub fn to_args(&self) -> Vec<OsString> {
        let mut args = vec!["-c:a".into(), self.codec().into()];
        if let Some(bitrate) = self.bitrate() {
            args.push("-b:a".into());
            args.push(bitrate.to_string().into());
        }
        args
    }
}

#[derive(Debug, Clone, Deref, DerefMut, Default, Getters, Setters)]
pub struct VideoOutputSettings {
    #[deref] #[deref_mut]
    common: CommonOutputStreamSettings,
    #[getset(get = "pub", set = "pub(self)")]
    crf: Option<String>,
}

impl VideoOutputSettings {
    pub fn to_args(&self) -> Vec<OsString> {
        let mut args = vec!["-c:v".into(), self.codec().into()];
        if let Some(bitrate) = self.bitrate() {
            args.push("-b:v".into());
            args.push(bitrate.to_string().into());
        }
        if let Some(crf) = self.crf() {
            args.push("-crf".into());
            args.push(crf.into());
        }
        args
    }
}

#[derive(Debug, Error)]
#[error("failed to build FFMpeg command: {0}")]
pub struct BuildCommandError(&'static str);

#[derive(Debug, Error)]
#[error("only one stdin input possible")]
pub struct CommandHasAlreadyOneStdinInput;

#[derive(Default, Getters, Clone)]
#[getset(get = "pub")]
pub struct CommandBuilder {
    bin_path: Option<PathBuf>,
    inputs: Vec<Input>,
    filters: Vec<Filter>,
    mappings: Vec<String>,
    video_output_settings: VideoOutputSettings,
    audio_output_settings: AudioOutputSettings,
    output: Option<PathBuf>,
    overwrite_output: bool,
}

impl CommandBuilder {

    pub fn set_ffmpeg_binary_path<P: AsRef<Path>>(&mut self, binary_path: P) {
        self.bin_path = Some(binary_path.as_ref().to_path_buf())
    }

    pub fn add_input_file_slice<P: AsRef<Path>>(&mut self, file_path: P, start: Option<Timestamp>, end: Option<Timestamp>) {
        self.inputs.push(Input::File { path: file_path.as_ref().to_path_buf(), start, end })
    }

    pub fn add_input_file<P: AsRef<Path>>(&mut self, file_path: P) {
        self.add_input_file_slice(file_path, None, None)
    }

    pub fn has_stdin_input(&self) -> bool {
        self.inputs().iter().any(|input| matches!(input, Input::StdinPipedRaw {..}))
    }

    pub fn add_stdin_input(&mut self, resolution: Resolution, frame_rate: u16) -> Result<(), CommandHasAlreadyOneStdinInput>  {
        if self.has_stdin_input() { return Err(CommandHasAlreadyOneStdinInput) }
        self.inputs.push(Input::StdinPipedRaw { resolution, frame_rate });
        Ok(())
    }

    pub fn add_audio_filter(&mut self, filter: &str) {
        self.filters.push(Filter::Audio(filter.to_string()));
    }

    pub fn add_video_filter(&mut self, filter: &str) {
        self.filters.push(Filter::Video(filter.to_string()));
    }

    pub fn add_complex_filter(&mut self, filter: &str) {
        self.filters.push(Filter::Complex(filter.to_string()));
    }

    pub fn add_mapping(&mut self, mapping: &str) {
        self.mappings.push(mapping.to_string());
    }

    pub fn add_mappings(&mut self, mappings: &[&str]) {
        self.mappings.append(&mut mappings.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    }

    pub fn set_output_video_codec(&mut self, codec: &str) {
        self.video_output_settings.set_codec(codec.to_string());
    }

    pub fn set_output_video_bitrate(&mut self, bitrate: Option<&str>) {
        self.video_output_settings.set_bitrate(bitrate.map(str::to_string));
    }

    pub fn set_output_video_crf(&mut self, crf: Option<&str>) {
        self.video_output_settings.set_crf(crf.map(str::to_string));
    }

    pub fn set_output_audio_codec(&mut self, codec: &str) {
        self.audio_output_settings.set_codec(codec.to_string());
    }

    pub fn set_output_audio_bitrate(&mut self, bitrate: Option<&str>) {
        self.audio_output_settings.set_bitrate(bitrate.map(str::to_string));
    }

    pub fn set_output_file<P: AsRef<Path>>(&mut self, file_path: P) {
        self.output = Some(file_path.as_ref().to_path_buf())
    }

    pub fn build(&self) -> Result<Command, BuildCommandError> {
        let binary_path = self.bin_path.clone().unwrap_or_else(|| PathBuf::from("ffmpeg"));
        let mut pcommand = process::Command::new(binary_path);

        if self.inputs.is_empty() { return Err(BuildCommandError("no input"))}
        for input in &self.inputs {
            pcommand.args(input.to_args());
        }

        for filter in &self.filters {
            pcommand.args(filter.to_args());
        }

        for mapping in &self.mappings {
            pcommand.arg("-map");
            pcommand.arg(OsString::from(mapping));
        }

        pcommand.args(self.audio_output_settings.to_args());
        pcommand.args(self.video_output_settings.to_args());

        if self.overwrite_output { pcommand.arg("-y"); }

        match &self.output {
            Some(output) => pcommand.arg(output),
            None => return Err(BuildCommandError("no output")),
        };

        Ok(Command { command: pcommand, debug: false, has_stdin_input: self.has_stdin_input() })
    }

}

#[derive(CopyGetters, Setters)]
pub struct Command {
    command: process::Command,
    #[getset(get_copy = "pub", set = "pub")]
    debug: bool,
    #[getset(get_copy = "pub")]
    has_stdin_input: bool,
}

#[derive(Debug, Error)]
#[error("failed to spawn ffmpeg process: {0}")]
pub struct SpawnError(#[from] IOError);

impl Command {

    fn spawn_base(mut self) -> Result<(process::Child, Option<process::ChildStdin>), SpawnError> {
        log::debug!("spawning process: {self}");
        let stdin_stdio = if self.has_stdin_input() { process::Stdio::piped() } else { process::Stdio::null() };
        let (stdout_stdio, stderr_stdio) = if self.debug() {
            (process::Stdio::inherit(), process::Stdio::inherit())
        } else {
            (process::Stdio::null(), process::Stdio::null())
        };
        let mut process_handle = self.command
            .stdin(stdin_stdio).stdout(stdout_stdio).stderr(stderr_stdio)
            .spawn()?;
        let process_stdin = if self.has_stdin_input() { process_handle.stdin.take() } else { None };
        Ok((process_handle, process_stdin))
    }

    pub fn spawn(self) -> Result<Process, SpawnError> {
        let (process_handle, process_stdin) = Self::spawn_base(self)?;
        Ok(Process::new(process_handle, process_stdin, None))
    }

    pub fn spawn_with_progress(self, frame_count: u64) -> Result<Process, SpawnError> {
        let (process_handle, process_stdin) = Self::spawn_base(self)?;
        Ok(Process::new(process_handle, process_stdin, Some(frame_count)))
    }

}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let components = [
                vec![self.command.get_program().to_string_lossy()],
                self.command.get_args().map(OsStr::to_string_lossy).collect::<Vec<_>>()
            ]
            .iter()
            .flatten()
            .map(|comp| {
                if comp.contains(' ') {
                    format!("\"{comp}\"")
                } else {
                    comp.to_string()
                }
            })
            .collect::<Vec<_>>();
        f.write_str(components.join(" ").as_str())
    }
}

#[derive(Debug, Error)]
#[error("ffmpeg process exited with an error: {exit_status}")]
pub struct ProcessError {
    exit_status: process::ExitStatus,
    stderr_content: Option<String>,
}

pub enum ProcessHandle {
    Process(process::Child),
    Monitor(JoinHandle<Result<(), ProcessError>>)
}

pub struct Process {
    handle: ProcessHandle,
    stdin: Option<process::ChildStdin>,
}

impl Process {

    fn new(handle: process::Child, stdin: Option<process::ChildStdin>, frame_count: Option<u64>) -> Self {
        let handle = match frame_count {
            Some(frame_count) => ProcessHandle::Monitor(tokio::spawn(Self::monitor(frame_count, handle))),
            None => ProcessHandle::Process(handle),
        };
        Process { handle, stdin }
    }

    async fn monitor(frame_count: u64, mut ffmpeg_child: process::Child) -> Result<(), ProcessError> {
        let mut ffmpeg_stderr = ffmpeg_child.stderr.take().unwrap();
        let mut output_buf = String::new();
        let mut read_buf = [0; 1024];
        let progress_style = ProgressStyle::with_template("{wide_bar} {percent:>3}% [ETA {eta:>3}]").unwrap();
        let progress_bar = ProgressBar::new(frame_count).with_style(progress_style);
        progress_bar.set_position(0);

        let exit_status = loop {

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

        if ! exit_status.success() {
            return Err(ProcessError { exit_status, stderr_content: None })
        }

        Ok(())
    }

    pub fn take_stdin(&mut self) -> Option<process::ChildStdin> {
        self.stdin.take()
    }

    pub async fn try_wait(&mut self) -> Result<bool, ProcessError> {
        use ProcessHandle::*;
        match &mut self.handle {
            Process(handle) => {
                match handle.try_wait().unwrap() {
                    Some(exit_status) =>
                        if exit_status.success() {
                            Ok(true)
                        } else {
                            Err(ProcessError { exit_status, stderr_content: None })
                        },
                    None => Ok(false),
                }
            }
            Monitor(handle) =>
                if handle.is_finished() {
                    match handle.await.unwrap() {
                        Ok(_) => Ok(true),
                        Err(process_error) => Err(process_error),
                    }
                } else {
                    Ok(false)
                }
        }
    }

    pub async fn wait(&mut self) -> Result<(), ProcessError> {
        use ProcessHandle::*;
        match &mut self.handle {
            Process(handle) =>
                match handle.wait().unwrap() {
                    exit_status if exit_status.success() => Ok(()),
                    exit_status => Err(ProcessError { exit_status, stderr_content: None })
                }
            Monitor(handle) => handle.await.unwrap()
        }
    }

}