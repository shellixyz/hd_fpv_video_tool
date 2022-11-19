
#![forbid(unsafe_code)]

use std::{
    path::PathBuf,
    process::exit,
    path::Path
};

use clap::{Parser, Subcommand, Args};
use derive_more::{From, Display, Error};

use hd_fpv_osd_font_tool::prelude::*;

use dji_fpv_video_tool::{
    osd::{
        dji::{
            font_dir::FontDir,
            file::{
                OpenError as OSDFileOpenError,
                Reader as OSDFileReader,
            },
        },
        frame_overlay::{
            DrawFrameOverlayError,
            Generator as OverlayGenerator,
            SaveFramesToDirError,
            Scaling,
            ScalingArgs,
        },
    },
    log_level::LogLevel
};


const DEFAULT_FONT_DIR: &str = "fonts";
const FONT_DIR_ENV_VAR_NAME: &str = "FONTS_DIR";

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {

    #[clap(short, long, value_parser, default_value_t = LogLevel::Info)]
    #[arg(value_enum)]
    log_level: LogLevel,

    #[command(subcommand)]
    command: Commands,

}

#[derive(Args)]
struct FontOptions {
    /// path to the directory containing font sets
    #[clap(short, long, value_parser, value_name = "dirpath")]
    font_dir: Option<PathBuf>,

    /// force using this font identifier when loading fonts, default is automatic
    #[clap(short = 'i', long, value_parser, value_name = "ident")]
    font_ident: Option<String>,
}

#[derive(Args)]
struct OSDArgs {
    /// shift frames to sync OSD with video
    #[clap(short = 'o', long, value_parser, value_name = "frames", default_value_t = 0)]
    frame_shift: i32,

    /// path to FPV.WTF .osd file
    osd_file: PathBuf,
}

#[derive(Subcommand)]
enum Commands {
    /// Displays information about the specified OSD file
    DisplayOSDFileInfo {
        osd_file: PathBuf,
    },

    /// Generates OSD overlay frames
    ///
    /// This command generates numbered OSD frame images from the specified WTF.FPV OSD file and writes
    /// them into the specified directory.
    ///
    /// Use this command when you want to generate OSD frame images to check what the OSD looks like
    /// or when you want to manually burn the OSD onto a video.
    ///
    /// Fonts are loaded either from the directory specified with the --font-dir option or
    /// from the directory found in the environment variable FONTS_DIR or
    /// if neither of these are available it falls back to the `fonts` directory inside the current directory
    GenerateOverlayFrames {

        #[clap(flatten)]
        scaling_args: ScalingArgs,

        #[clap(flatten)]
        font_options: FontOptions,

        #[clap(flatten)]
        osd_args: OSDArgs,

        /// directory in which the OSD frames will be written
        target_dir: PathBuf,
    },

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
        let refresh_percent_frames = frames.len() as f64 * 100.0 / *last_frame.index() as f64;
        let refresh_interval_frames = *last_frame.index() as f64 / frames.len() as f64;
        let refresh_interval_frames_str = match refresh_interval_frames.round() as u32 {
            1 => "every frame".to_owned(),
            frames => format!("every {frames} frames")
        };
        let refresh_freq = 60.0 / refresh_interval_frames;
        println!("OSD update rate: {refresh_percent_frames:.0}% of the video frames ({refresh_freq:.1}Hz or approximately {refresh_interval_frames_str})");
    }
    Ok(())
}

// if --font-ident was passed with a non-empty string return Some(Some(ident)) but if it was passed with an empty string return Some(None)
fn transform_font_ident<'a>(font_ident: &'a Option<&str>) -> Option<Option<&'a str>> {
    match font_ident {
        Some("") => Some(None),
        Some(font_ident_str) => Some(Some(font_ident_str)),
        None => None,
    }
}

fn prepare_overlay_generator(scaling_args: &ScalingArgs, font_options: &FontOptions, osd_args: &OSDArgs) -> anyhow::Result<OverlayGenerator> {
    let scaling = Scaling::try_from(scaling_args)?;
    let osd_file = OSDFileReader::open(&osd_args.osd_file)?;
    let font_dir_path = font_options.font_dir.clone().unwrap_or_else(|| PathBuf::from(std::env::var(FONT_DIR_ENV_VAR_NAME).unwrap_or_else(|_| DEFAULT_FONT_DIR.to_owned())));
    let font_dir = FontDir::new(&font_dir_path);
    let overlay_generator = osd_file.into_frame_overlay_generator(
        &font_dir,
        &transform_font_ident(&font_options.font_ident.as_deref()),
        scaling
    )?;
    Ok(overlay_generator)
}

fn generate_overlay_frames_command(command: &Commands) -> anyhow::Result<()> {
    if let Commands::GenerateOverlayFrames { scaling_args, font_options, osd_args, target_dir, } = command {
        let mut overlay_generator = prepare_overlay_generator(scaling_args, font_options, osd_args)?;
        overlay_generator.save_frames_to_dir(target_dir, osd_args.frame_shift)?;
    }
    Ok(())
}

fn main() {
    let cli = Cli::parse();

    pretty_env_logger::formatted_builder().parse_filters(cli.log_level.to_string().as_str()).init();

    let command_result = match &cli.command {
        command @ Commands::GenerateOverlayFrames {..} => generate_overlay_frames_command(command),
        Commands::DisplayOSDFileInfo { osd_file } => display_osd_file_info_command(osd_file),
    };

    if let Err(error) = command_result {
        log::error!("{}", error);
        exit(1);
    }
}
