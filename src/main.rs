use std::{process::exit, path::Path, fmt::Display, error::Error};

use clap::{Parser, Subcommand};
use hd_fpv_osd_font_tool::osd::standard_size_tile_container::StandardSizeTileArray;

use dji_fpv_video_tool::log_level::LogLevel;
use dji_fpv_video_tool::osd::file::{OpenError as OSDFileOpenError, Reader};

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

#[derive(Debug)]
enum GenerateOverlayError {
    OSDFileOpenError(OSDFileOpenError)
}

impl Error for GenerateOverlayError {}

impl From<OSDFileOpenError> for GenerateOverlayError {
    fn from(error: OSDFileOpenError) -> Self {
        Self::OSDFileOpenError(error)
    }
}

impl Display for GenerateOverlayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("error")
    }
}

fn generate_overlay<P: AsRef<Path>>(path: P) -> Result<(), GenerateOverlayError> {
    let osd_file = Reader::open(&path)?;
    let font_tiles = StandardSizeTileArray::load_from_bin_file("../hd_fpv_osd_font_tool/font_files/font_hd.bin").unwrap();
    // let mut overlay_generator = FrameOverlayGenerator::new(osd_file, &font_tiles).unwrap();
    let mut overlay_generator = osd_file.into_frame_overlay_generator(&font_tiles).unwrap();
    overlay_generator.save_frames_to_dir("/home/shel/fast_temp/osd_tiles").unwrap();

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
