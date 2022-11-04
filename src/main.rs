use std::{process::exit, path::Path, fmt::Display, error::Error};

use clap::{Parser, Subcommand};

use dji_fpv_video_tool::log_level::LogLevel;
use dji_fpv_video_tool::osd::file::{OpenError as OSDFileOpenError, Reader};
use dji_fpv_video_tool::osd::frame::*;

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

    // dbg!(osd_file.header());
    // let frame = osd_file.read_frame().unwrap();
    // dbg!(frame);
    // let mut counter = 0;
    // while let Ok(_frame) = osd_file.read_frame() {
    //     counter += 1;
    // }
    // let mut max_value = 0;
    // let frame = osd_file.read_frame().unwrap().unwrap();
    // let frame_size = frame.data.len();
    // for frame in osd_file {
    //     let frame = frame.unwrap();
    //     counter += 1;
    //     if frame.data.len() != frame_size {
    //         panic!("found {} != {}", frame.data.len(), frame_size);
    //     }
        // for y in 0..22 {
        //     for x in 50..60 {
        //         let pos = x + y * 60;
        //         if frame.data[pos] != 0 {
        //             panic!("oob frame {}, value {}", counter, frame.data[pos]);
        //         }
        //     }
        // }
    //     max_value = max_value.max(*frame.data.iter().max().unwrap());
    // }
    // println!("read {} frames, max value {}", counter, max_value);

    // let file_frame = osd_file.read_frame().unwrap().unwrap();
    // let frame_image = draw_frame(&file_frame);
    // frame_image.save("test_frame.png").unwrap();

    let fg = Generator::new();
    for (index, frame) in osd_file.into_iter().enumerate() {
        let frame_image = fg.draw_frame(&frame.unwrap());
        let path = format!("/home/shel/fast_temp/osd_tiles/{index:06}.png");
        frame_image.save(path).unwrap();
    }

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
