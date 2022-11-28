
#![forbid(unsafe_code)]

use std::{
    path::PathBuf,
    process::exit,
    path::Path
};

use clap::{Parser, Subcommand};
use derive_more::{From, Display, Error};

use hd_fpv_osd_font_tool::prelude::*;

use dji_fpv_video_tool::{prelude::*, cli::{transcode_video_args::TranscodeVideoOSDArgs, generate_overlay_args::GenerateOverlayArgs, start_end_args::StartEndArgs}, osd::overlay::OverlayVideoCodec};


#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {

    #[clap(short, long, value_parser, default_value_t = LogLevel::Info)]
    #[arg(value_enum)]
    log_level: LogLevel,

    #[command(subcommand)]
    command: Commands,

}

#[derive(Subcommand)]
enum Commands {
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
        output_dir: PathBuf,
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
        video_file: PathBuf,

        /// overwrite output file if it exists
        #[clap(short = 'y', long, value_parser)]
        overwrite: bool,
    },

    /// Cut video file
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

    }

}

#[derive(Debug, Error, From, Display)]
enum GenerateOverlayError {
    OSDFileOpen(OSDFileOpenError),
    BinFileLoad(BinFileLoadError),
    DrawFrameOverlay(DrawFrameOverlayError),
    SaveFramesToDir(SaveFramesToDirError),
}

fn display_osd_file_info_command<P: AsRef<Path>>(path: P) -> anyhow::Result<()> {
    let mut file = OSDFileReader::open(&path)?;
    let frames = file.frames()?;
    let header = file.header();
    println!();
    println!("Format version: {}", header.format_version());
    println!("OSD size: {} tiles", header.osd_dimensions());
    println!("OSD tiles dimension: {} px", header.tile_dimensions());
    println!("OSD video offset: {} px", header.offset());
    println!("OSD Font variant: {} ({})", header.font_variant_id(), header.font_variant());
    println!("Number of OSD frames: {}", frames.len());
    if let Some(last_frame) = frames.last() {
        println!("Highest video frame index: {}", last_frame.index());
        let refresh_percent_frames = frames.len() as f64 * 100.0 / last_frame.index() as f64;
        let refresh_interval_frames = last_frame.index() as f64 / frames.len() as f64;
        let refresh_interval_frames_str = match refresh_interval_frames.round() as u32 {
            1 => "every frame".to_owned(),
            frames => format!("every {frames} frames")
        };
        let refresh_freq = 60.0 / refresh_interval_frames;
        println!("OSD update rate: {refresh_percent_frames:.0}% of the video frames ({refresh_freq:.1}Hz or approximately {refresh_interval_frames_str})");
    }
    Ok(())
}

fn generate_overlay_prepare_generator(common_args: &GenerateOverlayArgs) -> anyhow::Result<OverlayGenerator> {
    let scaling = Scaling::try_from(common_args.scaling_args())?;
    let mut osd_file = OSDFileReader::open(common_args.osd_file())?;
    let font_dir = FontDir::new(&common_args.font_options().font_dir());
    let overlay_generator = OverlayGenerator::new(
        osd_file.frames()?,
        &font_dir,
        &common_args.font_options().font_ident(),
        scaling
    )?;
    Ok(overlay_generator)
}

fn generate_overlay_frames_command(command: &Commands) -> anyhow::Result<()> {
    if let Commands::GenerateOverlayFrames { common_args, output_dir: target_dir } = command {
        common_args.start_end().check_valid()?;
        let mut overlay_generator = generate_overlay_prepare_generator(common_args)?;
        overlay_generator.save_frames_to_dir(common_args.start_end().start(), common_args.start_end().end(), target_dir, common_args.frame_shift())?;
    }
    Ok(())
}

async fn generate_overlay_video_command(command: &Commands) -> anyhow::Result<()> {
    if let Commands::GenerateOverlayVideo { common_args, video_file, overwrite, codec } = command {
        common_args.start_end().check_valid()?;
        let mut overlay_generator = generate_overlay_prepare_generator(common_args)?;
        overlay_generator.generate_overlay_video(*codec, common_args.start_end().start(), common_args.start_end().end(), video_file, common_args.frame_shift(), *overwrite).await?;
    }
    Ok(())
}

async fn transcode_video_command(command: &Commands) -> anyhow::Result<()> {
    if let Commands::TranscodeVideo { osd_args, transcode_args } = command {

        transcode_args.start_end().check_valid()?;

        match osd_args.osd_file_path(transcode_args.input_video_file())? {
            Some(osd_file_path) => video::transcode_burn_osd(transcode_args, osd_file_path, osd_args).await?,
            None => video::transcode(transcode_args).await?,
        }
    }
    Ok(())
}

async fn fix_audio_command<P: AsRef<Path>, Q: AsRef<Path>>(input_video_file: P, output_video_file: &Option<Q>, overwrite: bool, sync: bool, volume: bool) -> anyhow::Result<()> {
    let fix_type = match (sync, volume) {
        (true, true) | (false, false) => VideoAudioFixType::SyncAndVolume,
        (true, false) => VideoAudioFixType::Sync,
        (false, true) => VideoAudioFixType::Volume,
    };
    video::fix_dji_air_unit_audio(input_video_file, output_video_file, overwrite, fix_type).await?;
    Ok(())
}



#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    pretty_env_logger::formatted_builder().parse_filters(cli.log_level.to_string().as_str()).init();

    let command_result = match &cli.command {

        command @ Commands::GenerateOverlayFrames {..} => generate_overlay_frames_command(command),
        command @ Commands::GenerateOverlayVideo {..} => generate_overlay_video_command(command).await,
        command @ Commands::TranscodeVideo {..} => transcode_video_command(command).await,
        Commands::DisplayOSDFileInfo { osd_file } => display_osd_file_info_command(osd_file),

        Commands::CutVideo { start_end, input_video_file, output_video_file, overwrite } =>
            video::cut(input_video_file, output_video_file, *overwrite, start_end).await.map_err(anyhow::Error::new),

        Commands::FixVideoAudio { input_video_file, output_video_file, overwrite, sync, volume } =>
            fix_audio_command(input_video_file, output_video_file, *overwrite, *sync, *volume).await,

        Commands::PlayVideoWithOSD { video_file, osd_video_file } =>
            video::play_with_osd(video_file, osd_video_file).map_err(anyhow::Error::new),
    };

    if let Err(error) = command_result {
        log::error!("{}", error);
        exit(1);
    }
}
