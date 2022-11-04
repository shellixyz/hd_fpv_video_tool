use std::{process::exit, path::Path, fmt::Display, error::Error};

use clap::{Parser, Subcommand};

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
    let mut osd_file = Reader::open(&path)?;

    dbg!(osd_file.header());
    // let frame = osd_file.read_frame().unwrap();
    // dbg!(frame);
    let mut counter = 0;
    // while let Ok(_frame) = osd_file.read_frame() {
    //     counter += 1;
    // }
    let frame = osd_file.read_frame().unwrap().unwrap();
    let frame_size = frame.data.len();
    for frame in osd_file {
        let frame = frame.unwrap();
        counter += 1;
        if frame.data.len() != frame_size {
            panic!("found {} != {}", frame.data.len(), frame_size);
        }
    }
    println!("read {} frames", counter);

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
