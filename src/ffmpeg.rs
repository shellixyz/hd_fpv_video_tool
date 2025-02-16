use std::{
	ffi::OsString,
	fmt::Display,
	io::{Error as IOError, Read, Write},
	os::unix::ffi::OsStrExt,
	path::{Path, PathBuf},
	process,
};

use derive_more::{Deref, DerefMut};
use getset::{CopyGetters, Getters, Setters};
use indicatif::{ProgressBar, ProgressStyle};
use lazy_static::lazy_static;
use path_absolutize::Absolutize;
use regex::Regex;
use ringbuffer::{self, ConstGenericRingBuffer, RingBufferExt, RingBufferWrite};
use tempfile::TempPath;
use thiserror::Error;
use tokio::task::JoinHandle;

use crate::{
	process::Command as ProcessCommand,
	video::{self, Resolution, Timestamp},
};

const DEFAULT_BINARY_PATH: &str = "ffmpeg";

#[derive(Debug, Clone)]
pub enum Input {
	File {
		path: PathBuf,
		start: Option<Timestamp>,
		end: Option<Timestamp>,
	},
	Filter {
		name: String,
		filter: String,
	},
	StdinPipedRaw {
		resolution: Resolution,
		frame_rate: u16,
	},
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
				args.append(
					&mut ["-f", "rawvideo", "-pix_fmt", "rgba", "-video_size"]
						.map(Into::into)
						.into(),
				);
				args.push(resolution.to_string().into());
				args.push("-r".into());
				args.push(frame_rate.to_string().into());
				args.append(&mut ["-i", "pipe:0"].map(Into::into).into());
			},

			Input::Filter { name, filter } => {
				args.append(&mut ["-f", name.as_str(), "-i", filter].map(Into::into).into());
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

#[derive(Debug, Clone, Getters, Setters, Default)]
#[getset(get = "pub", set = "pub(self)")]
pub struct CommonOutputStreamSettings {
	codec: Option<String>,
	bitrate: Option<String>,
}

#[derive(Debug, Clone, Deref, DerefMut, Default)]
pub struct AudioOutputSettings(CommonOutputStreamSettings);

impl AudioOutputSettings {
	pub fn to_args(&self) -> Vec<OsString> {
		let mut args = vec![];
		if let Some(codec) = self.codec() {
			args.push("-c:a".into());
			args.push(codec.into());
		}
		if let Some(bitrate) = self.bitrate() {
			args.push("-b:a".into());
			args.push(bitrate.to_string().into());
		}
		args
	}
}

#[derive(Debug, Clone, Deref, DerefMut, Default, Getters, Setters)]
pub struct VideoOutputSettings {
	#[deref]
	#[deref_mut]
	common: CommonOutputStreamSettings,
	#[getset(get = "pub", set = "pub(self)")]
	crf: Option<u8>,
}

impl VideoOutputSettings {
	pub fn to_args(&self) -> Vec<OsString> {
		let mut args = vec![];
		if let Some(codec) = self.codec() {
			args.push("-c:v".into());
			args.push(codec.into());
		}
		if let Some(bitrate) = self.bitrate() {
			args.push("-b:v".into());
			args.push(bitrate.to_string().into());
		}
		if let Some(crf) = self.crf() {
			args.push("-crf".into());
			args.push(crf.to_string().into());
		}
		args
	}
}

#[derive(Debug, Clone)]
pub enum Mapping {
	WithoutFilter(String),
	WithFilter { mapping: String, filter: Filter },
}

impl Mapping {
	pub fn to_args(&self) -> Vec<OsString> {
		let mut args = vec!["-map".into()];
		match self {
			Mapping::WithoutFilter(mapping) => args.push(mapping.into()),
			Mapping::WithFilter { mapping, filter } => {
				args.push(mapping.into());
				args.append(&mut filter.to_args())
			},
		}
		args
	}

	pub fn new_with_audio_filter(mapping: &str, filter: &str) -> Self {
		Self::WithFilter {
			mapping: mapping.to_string(),
			filter: Filter::Audio(filter.to_string()),
		}
	}

	pub fn new_with_video_filter(mapping: &str, filter: &str) -> Self {
		Self::WithFilter {
			mapping: mapping.to_string(),
			filter: Filter::Video(filter.to_string()),
		}
	}

	pub fn new_with_complex_filter(mapping: &str, filter: &str) -> Self {
		Self::WithFilter {
			mapping: mapping.to_string(),
			filter: Filter::Complex(filter.to_string()),
		}
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
	mappings: Vec<Mapping>,
	video_output_settings: VideoOutputSettings,
	audio_output_settings: AudioOutputSettings,
	// #[getset(skip)]
	// #[getset(get_copy = "pub")]
	// shortest: bool,
	args: Vec<String>,
	output: Option<PathBuf>,
	overwrite_output_file: bool,
}

impl CommandBuilder {
	pub fn set_ffmpeg_binary_path<P: AsRef<Path>>(&mut self, binary_path: P) -> &mut Self {
		self.bin_path = Some(binary_path.as_ref().to_path_buf());
		self
	}

	pub fn add_input_file_slice<P: AsRef<Path>>(
		&mut self,
		file_path: P,
		start: Option<Timestamp>,
		end: Option<Timestamp>,
	) -> &mut Self {
		self.inputs.push(Input::File {
			path: file_path.as_ref().to_path_buf(),
			start,
			end,
		});
		self
	}

	pub fn add_input_file<P: AsRef<Path>>(&mut self, file_path: P) -> &mut Self {
		self.add_input_file_slice(file_path, None, None);
		self
	}

	pub fn add_input_filter(&mut self, name: &str, filter: &str) -> &mut Self {
		self.inputs.push(Input::Filter {
			name: name.to_string(),
			filter: filter.to_string(),
		});
		self
	}

	pub fn has_stdin_input(&self) -> bool {
		self.inputs()
			.iter()
			.any(|input| matches!(input, Input::StdinPipedRaw { .. }))
	}

	pub fn add_stdin_input(
		&mut self,
		resolution: Resolution,
		frame_rate: u16,
	) -> Result<&mut Self, CommandHasAlreadyOneStdinInput> {
		if self.has_stdin_input() {
			return Err(CommandHasAlreadyOneStdinInput);
		}
		self.inputs.push(Input::StdinPipedRaw { resolution, frame_rate });
		Ok(self)
	}

	pub fn add_audio_filter(&mut self, filter: &str) -> &mut Self {
		self.filters.push(Filter::Audio(filter.to_string()));
		self
	}

	pub fn add_video_filter(&mut self, filter: &str) -> &mut Self {
		self.filters.push(Filter::Video(filter.to_string()));
		self
	}

	pub fn add_complex_filter(&mut self, filter: &str) -> &mut Self {
		self.filters.push(Filter::Complex(filter.to_string()));
		self
	}

	pub fn add_mapping(&mut self, mapping: &str) -> &mut Self {
		self.mappings.push(Mapping::WithoutFilter(mapping.to_string()));
		self
	}

	pub fn add_mapping_with_audio_filter(&mut self, mapping: &str, filter: &str) -> &mut Self {
		self.mappings.push(Mapping::new_with_audio_filter(mapping, filter));
		self
	}

	pub fn add_mapping_with_video_filter(&mut self, mapping: &str, filter: &str) -> &mut Self {
		self.mappings.push(Mapping::new_with_video_filter(mapping, filter));
		self
	}

	// NOTE: note sure a complex filter after map is valid
	pub fn add_mapping_with_complex_filter(&mut self, mapping: &str, filter: &str) -> &mut Self {
		self.mappings.push(Mapping::new_with_complex_filter(mapping, filter));
		self
	}

	pub fn add_mappings(&mut self, mappings: &[&str]) -> &mut Self {
		self.mappings.append(
			&mut mappings
				.iter()
				.map(|s| Mapping::WithoutFilter(s.to_string()))
				.collect::<Vec<_>>(),
		);
		self
	}

	pub fn set_output_video_codec(&mut self, codec: Option<&str>) -> &mut Self {
		self.video_output_settings.set_codec(codec.map(str::to_string));
		self
	}

	pub fn set_output_video_bitrate(&mut self, bitrate: Option<&str>) -> &mut Self {
		self.video_output_settings.set_bitrate(bitrate.map(str::to_string));
		self
	}

	pub fn set_output_video_crf(&mut self, crf: Option<u8>) -> &mut Self {
		self.video_output_settings.set_crf(crf);
		self
	}

	pub fn set_output_video_settings(
		&mut self,
		codec: Option<&str>,
		bitrate: Option<&str>,
		crf: Option<u8>,
	) -> &mut Self {
		self.set_output_video_codec(codec)
			.set_output_video_bitrate(bitrate)
			.set_output_video_crf(crf)
	}

	pub fn set_output_audio_codec(&mut self, codec: Option<&str>) -> &mut Self {
		self.audio_output_settings.set_codec(codec.map(str::to_string));
		self
	}

	pub fn set_output_audio_bitrate(&mut self, bitrate: Option<&str>) -> &mut Self {
		self.audio_output_settings.set_bitrate(bitrate.map(str::to_string));
		self
	}

	pub fn set_output_audio_settings(&mut self, codec: Option<&str>, bitrate: Option<&str>) -> &mut Self {
		self.set_output_audio_codec(codec).set_output_audio_bitrate(bitrate)
	}

	// pub fn set_shortest(&mut self, yes: bool) -> &mut Self {
	// 	self.shortest = yes;
	// 	self
	// }

	pub fn add_arg(&mut self, arg: &str) -> &mut Self {
		self.args.push(arg.to_string());
		self
	}

	pub fn add_args(&mut self, args: &[&str]) -> &mut Self {
		self.args
			.append(&mut args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>());
		self
	}

	pub fn set_overwrite_output_file(&mut self, yes: bool) -> &mut Self {
		self.overwrite_output_file = yes;
		self
	}

	pub fn set_output_file<P: AsRef<Path>>(&mut self, file_path: P) -> &mut Self {
		self.output = Some(file_path.as_ref().to_path_buf());
		self
	}

	pub fn build(&self) -> Result<Command, BuildCommandError> {
		let binary_path = self
			.bin_path
			.clone()
			.unwrap_or_else(|| PathBuf::from(DEFAULT_BINARY_PATH));
		let mut pcommand = ProcessCommand::new(binary_path);

		if self.inputs.is_empty() {
			return Err(BuildCommandError("no input"));
		}
		for input in &self.inputs {
			pcommand.args(input.to_args());
		}

		for filter in &self.filters {
			pcommand.args(filter.to_args());
		}

		for mapping in &self.mappings {
			pcommand.args(mapping.to_args());
		}

		pcommand.args(self.audio_output_settings.to_args());
		pcommand.args(self.video_output_settings.to_args());

		pcommand.args(self.args.iter().map(OsString::from).collect::<Vec<_>>());

		if self.overwrite_output_file {
			pcommand.arg("-y");
		}

		match &self.output {
			Some(output) => pcommand.arg(output),
			None => return Err(BuildCommandError("no output")),
		};

		Ok(Command {
			command: pcommand,
			has_stdin_input: self.has_stdin_input(),
		})
	}

	pub fn concat(
		binary_path: Option<&Path>,
		input_files: &[impl AsRef<Path>],
		output_file: impl AsRef<Path>,
		overwrite: bool,
	) -> Result<(TempPath, Command), BuildCommandError> {
		let binary_path = match binary_path {
			Some(path) => path.to_path_buf(),
			None => PathBuf::from(DEFAULT_BINARY_PATH),
		};
		let mut pcommand = ProcessCommand::new(binary_path);
		let mut temp_list_file =
			tempfile::NamedTempFile::new().map_err(|_| BuildCommandError("failed to create temp list file"))?;
		for input_file in input_files {
			let input_file = input_file
				.as_ref()
				.absolutize()
				.map_err(|_| BuildCommandError("failed to absolutize input file"))?;
			temp_list_file
				.write("file '".as_bytes())
				.map_err(|_| BuildCommandError("failed to write to temp list file"))?;
			temp_list_file
				.write(input_file.as_os_str().as_bytes())
				.map_err(|_| BuildCommandError("failed to write to temp list file"))?;
			temp_list_file
				.write("'\n".as_bytes())
				.map_err(|_| BuildCommandError("failed to write to temp list file"))?;
		}
		let temp_list_file_path = temp_list_file.into_temp_path();
		pcommand.args(["-f", "concat", "-safe", "0", "-i"]);
		pcommand.arg(temp_list_file_path.as_os_str());
		pcommand.args(["-c", "copy"]);
		pcommand.arg(output_file.as_ref());
		if overwrite {
			pcommand.arg("-y");
		}
		Ok((
			temp_list_file_path,
			Command {
				command: pcommand,
				has_stdin_input: false,
			},
		))
	}
}

pub struct ConcatCommand {
	command: Command,
	#[allow(dead_code)]
	temp_list_file: TempPath, // keep the temp file alive
}

impl Deref for ConcatCommand {
	type Target = Command;

	fn deref(&self) -> &Self::Target {
		&self.command
	}
}

#[derive(CopyGetters, Setters)]
pub struct Command {
	command: ProcessCommand,
	#[getset(get_copy = "pub")]
	has_stdin_input: bool,
}

#[derive(Debug, Default, Clone, CopyGetters)]
#[getset(get_copy = "pub")]
pub struct SpawnOptions {
	output_type: ProcessOutputType,
	priority: Option<i32>,
}

impl SpawnOptions {
	pub fn no_output(mut self) -> Self {
		self.output_type = ProcessOutputType::None;
		self
	}

	pub fn with_progress(mut self, frame_count: u64) -> Self {
		self.output_type = ProcessOutputType::Progress { frame_count };
		self
	}

	pub fn with_priority(mut self, priority: Option<i32>) -> Self {
		self.priority = priority;
		self
	}
}

#[derive(Debug, Error)]
#[error("failed spawning ffmpeg process: {bin_path}: {error}")]
pub struct SpawnError {
	bin_path: String,
	error: IOError,
}

impl Command {
	pub fn spawn(mut self, spawn_options: SpawnOptions) -> Result<Process, SpawnError> {
		log::debug!("spawning process: {self}");
		let stdin_stdio = if self.has_stdin_input() {
			process::Stdio::piped()
		} else {
			process::Stdio::null()
		};
		let (stdout_stdio, stderr_stdio) = match spawn_options.output_type {
			ProcessOutputType::Inherited => (process::Stdio::inherit(), process::Stdio::inherit()),
			ProcessOutputType::Progress { .. } | ProcessOutputType::None => {
				(process::Stdio::null(), process::Stdio::piped())
			},
		};
		let mut process_handle = self
			.command
			.stdin(stdin_stdio)
			.stdout(stdout_stdio)
			.stderr(stderr_stdio)
			.spawn()
			.map_err(|error| SpawnError {
				error,
				bin_path: self.command.get_program().to_string_lossy().to_string(),
			})?;

		if let Some(priority) = spawn_options.priority {
			unsafe {
				if libc::setpriority(libc::PRIO_PROCESS, process_handle.id(), priority) != 0 {
					log::error!("failed to set ffmpeg process priority to {}", priority);
				}
			}
		}

		let process_stdin = if self.has_stdin_input() {
			process_handle.stdin.take()
		} else {
			None
		};
		Ok(Process::new(process_handle, process_stdin, spawn_options.output_type))
	}

	// pub fn spawn(self) -> Result<Process, SpawnError> {
	// 	self.spawn_base(ProcessOutputType::Inherited)
	// }

	// pub fn spawn_no_output(self) -> Result<Process, SpawnError> {
	// 	self.spawn_base(ProcessOutputType::None)
	// }

	// pub fn spawn_with_progress(self, frame_count: u64) -> Result<Process, SpawnError> {
	// 	let output_type = if frame_count == 0 {
	// 		ProcessOutputType::None
	// 	} else {
	// 		ProcessOutputType::Progress { frame_count }
	// 	};
	// 	self.spawn_base(output_type)
	// }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ProcessOutputType {
	#[default]
	Inherited,
	Progress {
		frame_count: u64,
	},
	None,
}

impl Display for Command {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.command.fmt(f)
	}
}

#[derive(Debug, Getters, Error)]
#[getset(get = "pub")]
pub struct ProcessError {
	exit_status: process::ExitStatus,
	stderr_content: Option<String>,
}

impl Display for ProcessError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "ffmpeg process exited with an error: {}", self.exit_status)?;
		if let Some(stderr_content) = &self.stderr_content {
			f.write_str("\n\nFFMpeg last lines:\n\n")?;
			f.write_str(stderr_content)?;
		}
		Ok(())
	}
}

pub struct Process {
	handle: process::Child,
	monitor_handle: Option<JoinHandle<Vec<String>>>,
	stdin: Option<process::ChildStdin>,
}

impl Process {
	fn new(mut handle: process::Child, stdin: Option<process::ChildStdin>, output_type: ProcessOutputType) -> Self {
		let monitor_handle = match output_type {
			ProcessOutputType::Inherited => None,
			ProcessOutputType::Progress { frame_count } => Some(tokio::spawn(Self::monitor(
				handle.stderr.take().unwrap(),
				Some(frame_count),
			))),
			ProcessOutputType::None => Some(tokio::spawn(Self::monitor(handle.stderr.take().unwrap(), None))),
		};
		Process {
			handle,
			monitor_handle,
			stdin,
		}
	}

	async fn monitor(mut ffmpeg_stderr: process::ChildStderr, frame_count: Option<u64>) -> Vec<String> {
		let mut output_buf = String::new();
		let mut read_buf = [0; 1024];
		let mut last_lines = ConstGenericRingBuffer::<_, 16>::new();

		let progress_bar = frame_count.map(|frame_count| {
			#[allow(clippy::literal_string_with_formatting_args)]
			let progress_style = ProgressStyle::with_template("{wide_bar} {percent:>3}% [ETA {eta:>3}]").unwrap();
			let progress_bar = ProgressBar::new(frame_count).with_style(progress_style);
			progress_bar.set_position(0);
			progress_bar
		});

		loop {
			let read_count = ffmpeg_stderr.read(&mut read_buf).unwrap();
			if read_count == 0 {
				break;
			}
			output_buf.push_str(String::from_utf8_lossy(&read_buf[0..read_count]).to_string().as_str());

			let mut lines = output_buf.split_inclusive('\n').map(str::to_string);
			let last_line = lines.next_back().unwrap();

			let last_cr_lines = last_line.split_inclusive('\r').map(str::to_string).collect::<Vec<_>>();

			if let Some(progress_bar) = &progress_bar {
				if let Some(cr_line) = last_cr_lines.iter().rfind(|cr_pl| cr_pl.ends_with('\r')) {
					lazy_static! {
						static ref PROGRESS_RE: Regex = Regex::new(r"\Aframe=\s*(\d+)").unwrap();
					}
					if let Some(captures) = PROGRESS_RE.captures(cr_line) {
						let frame: u64 = captures.get(1).unwrap().as_str().parse().unwrap();
						progress_bar.set_position(frame);
					}
				}
			}

			last_lines.extend(lines);
			output_buf.clear();

			if last_line.ends_with('\n') {
				last_lines.push(last_line);
			} else {
				let last_cr_line = last_cr_lines.last().unwrap();
				if !last_cr_line.ends_with('\r') {
					output_buf.push_str(last_cr_line);
				}
			}
		}

		if let Some(progress_bar) = progress_bar {
			progress_bar.set_position(frame_count.unwrap());
			progress_bar.finish_and_clear();
		}

		last_lines.to_vec()
	}

	pub fn take_stdin(&mut self) -> Option<process::ChildStdin> {
		self.stdin.take()
	}

	pub fn id(&self) -> u32 {
		self.handle.id()
	}

	async fn last_output_lines(&mut self) -> Option<String> {
		match self.monitor_handle.take() {
			Some(monitor_handle) => Some(monitor_handle.await.unwrap().concat()),
			None => None,
		}
	}

	pub async fn try_wait(&mut self) -> Result<bool, ProcessError> {
		match self.handle.try_wait().unwrap() {
			Some(exit_status) => {
				if exit_status.success() {
					Ok(true)
				} else {
					Err(ProcessError {
						exit_status,
						stderr_content: self.last_output_lines().await,
					})
				}
			},
			None => Ok(false),
		}
	}

	pub async fn wait(&mut self) -> Result<(), ProcessError> {
		match self.handle.wait().unwrap() {
			exit_status if exit_status.success() => Ok(()),
			exit_status => Err(ProcessError {
				exit_status,
				stderr_content: self.last_output_lines().await,
			}),
		}
	}

	pub fn kill(mut self) -> Result<(), IOError> {
		self.handle.kill()
	}
}

impl video::Region {
	pub fn to_ffmpeg_filter_string(&self) -> String {
		format!(
			"x={}:y={}:w={}:h={}",
			self.top_left_corner().x,
			self.top_left_corner().y,
			self.dimensions().width,
			self.dimensions().height
		)
	}
}
