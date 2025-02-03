use std::path::PathBuf;

use clap::{Parser, Subcommand};
use getset::CopyGetters;
use hd_fpv_video_tool::{cli::start_end_args::CutVideoStartEndArgs, prelude::*};

use crate::shell_autocompletion::*;

/// hd_fpv_video_tool is a command line tool for manipulating video files and OSD files recorded with the DJI and Walksnail Avatar FPV systems
///
/// Author: Michel Pastor <shellixyz@gmail.com>
///
/// Each command is aliased to the concatenation of the first letter of each word of the command{n}
/// Example: the `generate-overlay-frames` command is aliased to `gof`
#[derive(Parser, CopyGetters)]
#[clap(version, about, long_about)]
pub struct Cli {
	#[clap(short, long, value_parser, default_value_t = LogLevel::Info)]
	#[arg(value_enum)]
	#[getset(get_copy = "pub")]
	log_level: LogLevel,

	#[command(subcommand)]
	pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
	/// Display information about the specified OSD file
	#[clap(alias = "dofi")]
	DisplayOSDFileInfo { osd_file: PathBuf },

	/// Generate a transparent overlay frame sequence as PNG files from a .osd file
	///
	/// This command generates numbered OSD frame images from the specified WTF.FPV OSD file and writes
	/// them into the specified output directory.
	///
	/// Use this command when you want to generate OSD frame images to check what the OSD looks like
	/// or when you want to manually burn the OSD onto a video.
	///
	/// If you specify a target resolution with --target-resolution or a video file to read the resolution from
	/// with --target-video-file then the kind of tiles (HD/SD) to use and whether to use scaling or not
	/// will be decided to best match the target video resolution and to get the best OSD sharpness.
	/// If neither of these options are specified no scaling will be used and the kind of tiles used will be
	/// the native kind of tiles corresponding to the kind of OSD layout read from the FPV.WTF .osd file.
	///
	/// Fonts are loaded either from the directory specified with the --font-dir option or
	/// from the directory found in the environment variable FONTS_DIR or
	/// if neither of these are available it falls back to the `fonts` directory inside the current directory.
	#[clap(alias = "gof")]
	GenerateOverlayFrames {
		#[clap(flatten)]
		common_args: GenerateOverlayArgs,

		/// directory in which the OSD frames will be written
		output_dir: Option<PathBuf>,
	},

	/// Generate an OSD overlay video to be displayed over another video
	///
	/// This command generates a transparent video with the OSD frames rendered from the specified WTF.FPV OSD file.
	/// The generated video can then be used to play an FPV video with OSD without having to burn the OSD into the video.
	///
	/// If you specify a target resolution with --target-resolution or a video file to read the resolution from
	/// with --target-video-file then the kind of tiles (HD/SD) to use and whether to use scaling or not
	/// will be decided to best match the target video resolution and to get the best OSD sharpness.
	/// If neither of these options are specified no scaling will be used and the kind of tiles used will be
	/// the native kind of tiles corresponding to the kind of OSD layout read from the FPV.WTF .osd file.
	///
	/// VP8 or VP9 codecs can be selected with the --codec option. Files generated with the VP9 codec are smaller
	/// but also it is roughly twice as slow as encoding with the VP8 codec which is already unfortunately pretty slow.
	///
	/// Fonts are loaded either from the directory specified with the --font-dir option or
	/// from the directory found in the environment variable FONTS_DIR or
	/// if neither of these are available it falls back to the `fonts` directory inside the current directory.
	///
	/// NOTE: unfortunately this is very slow right now because only a handful of video formats support transparency
	/// and their encoders are very slow
	#[clap(alias = "gov")]
	GenerateOverlayVideo {
		#[clap(flatten)]
		common_args: GenerateOverlayArgs,

		#[clap(short = 'P', long)]
		ffmpeg_priority: Option<i32>,

		#[clap(short, long, default_value = "vp8")]
		codec: OverlayVideoCodec,

		/// path of the video file to generate
		video_file: Option<PathBuf>,

		/// overwrite output file if it exists
		#[clap(short = 'y', long, value_parser)]
		overwrite: bool,
	},

	/// Cut a video file without transcoding by specifying the desired start and/or end timestamp
	///
	/// Note that without transcoding videos can only be cut at the nearest P-frame so the cuts may not
	/// be at exactly the start/end points. If you need precise slicing use the `transcode` command instead.
	#[clap(alias = "cv")]
	CutVideo {
		#[clap(flatten)]
		start_end: CutVideoStartEndArgs,

		#[clap(short = 'P', long)]
		ffmpeg_priority: Option<i32>,

		/// input video file path
		input_video_file: PathBuf,

		/// output video file path
		output_video_file: Option<PathBuf>,

		/// overwrite output file if it exists
		#[clap(short = 'y', long, value_parser)]
		overwrite: bool,
	},

	/// Fix a DJI Air Unit video's audio sync and/or volume
	///
	/// If the output video file is not provided the output video will be written in the same directory
	/// as the input video with the same file name with suffix `_fixed_audio`
	///
	/// Note that fixing the audio/video sync will only work if the start of the original video from
	/// the DJI FPV air unit has NOT been cut off.
	#[clap(alias = "fva")]
	FixVideoAudio {
		/// fix audio sync only
		#[clap(short, long, value_parser)]
		sync: bool,

		/// fix audio volume only
		#[clap(short, long, value_parser)]
		volume: bool,

		#[clap(short = 'P', long)]
		ffmpeg_priority: Option<i32>,

		/// input video file path
		input_video_file: PathBuf,

		/// output video file path
		output_video_file: Option<PathBuf>,

		/// overwrite output file if it exists
		#[clap(short = 'y', long, value_parser)]
		overwrite: bool,
	},

	/// Transcode a video file, optionally burning the OSD onto it
	///
	/// Fonts are loaded either from the directory specified with the --font-dir option or
	/// from the directory found in the environment variable FONTS_DIR or
	/// if neither of these are available it falls back to the `fonts` directory inside the current directory
	#[clap(alias = "tv")]
	TranscodeVideo {
		#[clap(flatten)]
		osd_args: TranscodeVideoOSDArgs,

		#[clap(flatten)]
		transcode_args: TranscodeVideoArgs,
	},

	/// Play a video with OSD by overlaying a transparent OSD video in real time
	///
	/// You can generate a compatible OSD overlay video file with the `generate-overlay-video` command.
	///
	/// If the <OSD_VIDEO_FILE> argument is not provided it will try to use the file with the same base name
	/// as the <VIDEO_FILE> argument with suffix `_osd` and with `webm` extension.
	#[clap(alias = "pvwo")]
	PlayVideoWithOSD {
		video_file: PathBuf,

		osd_video_file: Option<PathBuf>,
	},

	/// Splice videos files together
	#[clap(alias = "sv")]
	SpliceVideos {
		#[clap(short = 'P', long)]
		ffmpeg_priority: Option<i32>,

		/// input video files
		input_video_files: Vec<PathBuf>,

		/// output video file path
		// #[clap(short, long)]
		output: PathBuf,

		/// overwrite output file if it exists
		#[clap(short = 'y', long, value_parser)]
		overwrite: bool,
	},

	/// Add a silent audio stream to a video file
	///
	/// Useful when the input video does not have an audio stream and you want to splice it with other videos
	/// that do have audio and you want to keep the audio from the other videos
	#[clap(alias = "aas")]
	AddAudioStream {
		/// audio encoder to use
		///
		/// This value is directly passed to the `-c:a` FFMpeg argument.{n}
		/// Run `ffmpeg -encoders` for a list of available encoders
		#[clap(long, value_parser, default_value = "aac")]
		audio_encoder: String,

		/// max audio bitrate
		#[clap(long, value_parser, default_value = "93k")]
		audio_bitrate: String,

		#[clap(short = 'P', long)]
		ffmpeg_priority: Option<i32>,

		/// input video file path
		input_video_file: PathBuf,

		/// output video file path
		output_video_file: Option<PathBuf>,

		/// overwrite output file if it exists
		#[clap(short = 'y', long, value_parser)]
		overwrite: bool,
	},

	#[clap(hide(true))]
	GenerateShellAutocompletionFiles {
		#[clap(value_parser = generate_shell_autocompletion_files_arg_parser)]
		shell: GenerateShellAutoCompletionFilesArg,
	},

	#[clap(hide(true))]
	GenerateManPages,
}
