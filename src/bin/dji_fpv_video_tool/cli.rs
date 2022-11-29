
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use dji_fpv_video_tool::prelude::*;
use getset::CopyGetters;

use crate::shell_autocompletion::*;


/// A tool to manipulate DJI video files and generate OSD frames from FPV.WTF .osd files
///
/// Each command is aliased to the concatenation of the first letter of each word of the command{n}
/// Example: the `generate-overlay-frames` command is aliased to `gof`
#[derive(Parser, CopyGetters)]
#[clap(author, version, about, long_about = None)]
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
    /// Displays information about the specified OSD file
    #[clap(alias = "dofi")]
    DisplayOSDFileInfo {
        osd_file: PathBuf,
    },

    /// Generates OSD overlay frames
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

    /// Generates OSD overlay video
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

        #[clap(short, long, default_value = "vp8")]
        codec: OverlayVideoCodec,

        /// path of the video file to generate
        video_file: Option<PathBuf>,

        /// overwrite output file if it exists
        #[clap(short = 'y', long, value_parser)]
        overwrite: bool,
    },

    /// Cut video file
    ///
    /// Note that without transcoding videos can only be cut at the nearest P-frame so the cuts may not
    /// be at exactly the start/end points. If you need precise slicing use the `transcode` command instead.
    #[clap(alias = "cv")]
    CutVideo {

        #[clap(flatten)]
        start_end: StartEndArgs,

        /// input video file path
        input_video_file: PathBuf,

        /// output video file path
        output_video_file: Option<PathBuf>,

        /// overwrite output file if it exists
        #[clap(short = 'y', long, value_parser)]
        overwrite: bool,
    },

    /// Fixes DJI Air Unit video audio sync and/or volume
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

        /// input video file path
        input_video_file: PathBuf,

        /// output video file path
        output_video_file: Option<PathBuf>,

        /// overwrite output file if it exists
        #[clap(short = 'y', long, value_parser)]
        overwrite: bool,
    },

    /// Transcodes video file optionally burning OSD onto it
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

    /// Play video using MPV video player with OSD by overlaying transparent OSD video in real time
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

    #[clap(hide(true))]
    GenerateShellAutocompletionFiles {
        #[clap(value_parser = generate_shell_autocompletion_files_arg_parser)]
        shell: GenerateShellAutoCompletionFilesArg,
    },

    #[clap(hide(true))]
    GenerateManPages,

}