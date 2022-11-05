use std::fmt::Display;
use std::{process::exit, path::Path};

use clap::{Parser, Subcommand};
use derive_more::{From, Display, Error};
use dji_fpv_video_tool::osd::frame_overlay::{DrawFrameOverlayError, link_missing_frames};
use hd_fpv_osd_font_tool::osd::standard_size_tile_container::StandardSizeTileArray;
use hd_fpv_osd_font_tool::osd::bin_file::LoadError as BinFileLoadError;

use dji_fpv_video_tool::log_level::LogLevel;
use dji_fpv_video_tool::osd::file::{OpenError as OSDFileOpenError, Reader, SaveFramesToDirError};

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

fn generate_overlay<P: AsRef<Path> + Display>(path: P) -> Result<(), GenerateOverlayError> {
    let mut osd_file = Reader::open(&path)?;
    let font_tiles = StandardSizeTileArray::load_from_bin_file("../hd_fpv_osd_font_tool/font_files/font_hd.bin")?;
    let mut overlay_generator = osd_file.into_frame_overlay_generator(&font_tiles)?;
    overlay_generator.save_frames_to_dir("/home/shel/fast_temp/osd_tiles")?;
    // link_missing_frames("/home/shel/fast_temp/osd_tiles", osd_file.frames().unwrap().into_iter().map(|x| *x.index()).collect()).unwrap();

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
}
