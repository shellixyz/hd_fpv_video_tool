
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

fn generate_overlay<P: AsRef<Path>>(path: P) -> Result<(), GenerateOverlayError> {
    let osd_file = OSDFileReader::open(&path)?;
    let tile_set = bin_file::load_set_norm("../hd_fpv_osd_font_tool/font_files", &None).unwrap();
    let mut overlay_generator = osd_file.into_frame_overlay_generator(&tile_set, TargetResolution::TrGoggles4By3, Scale::Yes { minimum_horizontal_margin: 30, minimum_vertical_margin: 30 })?;
    overlay_generator.save_frames_to_dir("/home/shel/fast_temp/osd_tiles", 0)?;
    Ok(())
}

fn main() {
    let cli = Cli::parse();

    pretty_env_logger::formatted_builder().parse_filters(cli.log_level.to_string().as_str()).init();

    let command_result = match &cli.command {
        Commands::GenerateOverlay { osd_file } => generate_overlay(osd_file)
    };

    if let Err(error) = command_result {
        log::error!("{}", error);
        exit(1);
    }
    // let vid_resolution = dji_fpv_video_tool::osd::frame_overlay::VideoResolution::new(960, 720);
    // println!("{:?}", dji_fpv_video_tool::osd::dji::Kind::HD.best_kind_of_tiles_to_use_without_scaling(vid_resolution));

    // let vid_resolution = dji_fpv_video_tool::osd::frame_overlay::VideoResolution::new(1280, 720);
    // println!("{:?}", dji_fpv_video_tool::osd::dji::Kind::HD.best_kind_of_tiles_to_use_with_scaling(vid_resolution));
}
