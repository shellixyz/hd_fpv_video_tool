
#![forbid(unsafe_code)]

use std::{process::exit, path::Path};

use clap::{Parser, Subcommand};
use derive_more::{From, Display, Error};
use dji_fpv_video_tool::osd::frame_overlay::{DrawFrameOverlayError, SaveFramesToDirError, TargetResolution, Scale};
use hd_fpv_osd_font_tool::osd::bin_file::{LoadError as BinFileLoadError, self};

use dji_fpv_video_tool::log_level::LogLevel;
use dji_fpv_video_tool::osd::dji::file::{OpenError as OSDFileOpenError, Reader as OSDFileReader};

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
    DisplayOSDFileInfo {
        osd_file: String,
    },
    GenerateOverlay {
        osd_file: String,
    }
}

#[derive(Debug, Error, From, Display)]
enum GenerateOverlayError {
    OSDFileOpen(OSDFileOpenError),
    BinFileLoad(BinFileLoadError),
    DrawFrameOverlay(DrawFrameOverlayError),
    SaveFramesToDir(SaveFramesToDirError),
}

fn display_osd_file_info<P: AsRef<Path>>(path: P) -> anyhow::Result<()> {
    let mut file = OSDFileReader::open(&path)?;
    let frames = file.frames()?;
    let header = file.header();
    println!();
    println!("Format version: {}", header.format_version());
    println!("OSD size: {} tiles", header.osd_dimensions());
    println!("OSD tiles dimension: {} px", header.tile_dimensions());
    println!("OSD video offset: {} px", header.offset());
    println!("OSD Font variant: {} ({})", header.font_variant_id(), header.font_variant_string());
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

fn generate_overlay<P: AsRef<Path>>(path: P) -> anyhow::Result<()> {
    let osd_file = OSDFileReader::open(&path)?;
    let tile_set = bin_file::load_set_norm("../hd_fpv_osd_font_tool/font_files", &None).unwrap();
    let mut overlay_generator = osd_file.into_frame_overlay_generator(&tile_set, TargetResolution::TrAU4by3, Scale::Yes { minimum_horizontal_margin: 30, minimum_vertical_margin: 30 })?;
    // let mut overlay_generator = osd_file.into_frame_overlay_generator(&tile_set, TargetResolution::Tr720p, Scale::No)?;
    overlay_generator.save_frames_to_dir("/home/shel/fast_temp/osd_tiles", 0)?;
    Ok(())
}

fn main() {
    let cli = Cli::parse();

    pretty_env_logger::formatted_builder().parse_filters(cli.log_level.to_string().as_str()).init();

    let command_result = match &cli.command {
        Commands::GenerateOverlay { osd_file } => generate_overlay(osd_file),
        Commands::DisplayOSDFileInfo { osd_file } => display_osd_file_info(osd_file),
    };

    if let Err(error) = command_result {
        log::error!("{}", error);
        exit(1);
    }
}
