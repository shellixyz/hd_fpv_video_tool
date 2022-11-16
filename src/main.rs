
#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::{process::exit, path::Path};

use clap::{Parser, Subcommand};
use derive_more::{From, Display, Error};
use dji_fpv_video_tool::osd::frame_overlay::{DrawFrameOverlayError, SaveFramesToDirError, TargetResolution, Scaling};
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
        osd_file: PathBuf,
    },
    GenerateOverlay {
        /// force using scaling, default is automatic
        #[clap(short, long, value_parser)]
        scaling: bool,

        /// force disable scaling, default is automatic
        #[clap(short, long, value_parser)]
        no_scaling: bool,

        /// minimum margins to decide whether scaling should be used and how much to scale
        #[clap(long, value_parser, value_name = "horizontal:vertical", default_value = "20:20")]
        min_margins: String,

        /// minimum percentage of OSD coverage under which scaling will be used if --scaling/--no-scaling options are not provided
        #[clap(long, value_parser = clap::value_parser!(u8).range(0..=100), value_name = "percent", default_value = "90")]
        min_coverage: u8,

        /// path to the directory containing font sets
        #[clap(short, long, value_parser, value_name = "dirpath", default_value = "fonts")]
        font_dir: String,

        /// force using this font identifier when loading fonts, default is automatic
        #[clap(short = 'i', long, value_parser, value_name = "ident")]
        font_ident: Option<String>,

        /// shift frames to sync OSD with video
        #[clap(short = 'o', long, value_parser, value_name = "frames", default_value_t = 0)]
        frame_shift: i32,

        /// path to FPV.WTF .osd file
        osd_file: PathBuf,

        /// valid values are 720p, 720p4:3, 1080p, 1080p4:3 or a custom resolution in the format <width>x<height>
        target_video_resolution: String,

        /// directory in which the OSD frames will be written
        target_dir: PathBuf,
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

fn generate_overlay(command: &Commands) -> anyhow::Result<()> {
    if let Commands::GenerateOverlay {
            scaling, no_scaling, min_margins, font_dir, osd_file, target_video_resolution,
            target_dir, min_coverage, font_ident, frame_shift: frame_offset
        } = command {
            let osd_file = OSDFileReader::open(osd_file)?;
            let target_video_resolution = TargetResolution::try_from(target_video_resolution.as_str())?;
            let scaling = Scaling::try_from(*scaling, *no_scaling, min_margins, *min_coverage, target_video_resolution)?;
            let tile_set = bin_file::load_set_norm(font_dir, &font_ident.as_deref()).unwrap();
            let mut overlay_generator = osd_file.into_frame_overlay_generator(&tile_set, target_video_resolution, scaling)?;
            overlay_generator.save_frames_to_dir(target_dir, *frame_offset)?;
    }
    Ok(())
}

fn main() {
    let cli = Cli::parse();

    pretty_env_logger::formatted_builder().parse_filters(cli.log_level.to_string().as_str()).init();

    let command_result = match &cli.command {
        command @ Commands::GenerateOverlay {..} => generate_overlay(command),
        Commands::DisplayOSDFileInfo { osd_file } => display_osd_file_info(osd_file),
    };

    if let Err(error) = command_result {
        log::error!("{}", error);
        exit(1);
    }
}
