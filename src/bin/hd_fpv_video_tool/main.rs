
#![forbid(unsafe_code)]

use std::{
    io::Write,
    process::exit,
    path::{Path, PathBuf},
    env::current_exe,
};

use clap::Parser;
use env_logger::fmt::Color;
use strum::IntoEnumIterator;

use anyhow::anyhow;


use hd_fpv_video_tool::{prelude::*, osd::file::GenericReader};
mod shell_autocompletion;
mod man_pages;
mod cli;

use {cli::*, man_pages::*, shell_autocompletion::*};


fn display_osd_file_info_command<P: AsRef<Path>>(path: P) -> anyhow::Result<()> {
    let mut reader = osd::file::open(path)?;

    println!();
    match &reader {
        osd::file::Reader::DJI(reader) => {
            let header = reader.header();
            println!("OSD file type: DJI FPV");
            println!("Format version: {}", header.format_version());
            println!("OSD size: {} tiles", header.osd_dimensions());
            println!("OSD tiles dimension: {} px", header.tile_dimensions());
            println!("OSD video offset: {} px", header.offset());
            println!("OSD Font variant: {} ({})", header.font_variant_id(), header.font_variant());
        },
        osd::file::Reader::WSA(reader) => {
            let header = reader.header();
            println!("OSD file type: Walksnail Avatar");
            println!("OSD Font variant: {} ({})", header.font_variant_id(), header.font_variant());
        },
    }

    let frames = reader.frames()?;
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
    let scaling = Scaling::try_from_scaling_args(common_args.scaling_args(), common_args.target_video_file())?;
    let mut osd_file_reader = osd::file::open(common_args.osd_file())?;
    let font_dir = FontDir::new(common_args.font_options().font_dir()?);
    let overlay_generator = OverlayGenerator::new(
        osd_file_reader.frames()?,
        osd_file_reader.font_variant(),
        &font_dir,
        &common_args.font_options().font_ident(),
        scaling,
        common_args.hide_regions(),
        common_args.hide_items()
    )?;
    Ok(overlay_generator)
}

fn generate_overlay_frames_command(command: &Commands) -> anyhow::Result<()> {
    if let Commands::GenerateOverlayFrames { common_args, output_dir } = command {
        common_args.check_valid()?;
        let output_dir = match (output_dir, common_args.target_video_file()) {
            (Some(output_dir), _) => output_dir.clone(),
            (None, Some(target_video_file)) => {
                let target_video_file_stem = target_video_file.file_stem().ok_or_else(|| anyhow!("target video file has no file name"))?;
                let mut output_file_stem = target_video_file_stem.to_os_string();
                output_file_stem.push("_osd_frames");
                PathBuf::from(output_file_stem)
            },
            (None, None) => {
                let osd_file = common_args.osd_file();
                let mut output_dir_name = Path::new(osd_file.file_stem().ok_or_else(|| anyhow!("OSD file has no file name"))?).as_os_str().to_os_string();
                output_dir_name.push("_osd_frames");
                osd_file.with_file_name(output_dir_name)
            }
        };
        let mut overlay_generator = generate_overlay_prepare_generator(common_args)?;
        overlay_generator.save_frames_to_dir(common_args.start_end().start(), common_args.start_end().end(), output_dir, common_args.frame_shift()?)?;
    }
    Ok(())
}

async fn generate_overlay_video_command(command: &Commands) -> anyhow::Result<()> {
    if let Commands::GenerateOverlayVideo { common_args, video_file, overwrite, codec } = command {
        common_args.check_valid()?;
        let output_video_path = match (video_file, common_args.target_video_file()) {
            (Some(output_video_file), _) => output_video_file.clone(),
            (None, Some(target_video_file)) => {
                let target_video_file_stem = target_video_file.file_stem().ok_or_else(|| anyhow!("target video file has no file name"))?;
                let mut output_file_stem = target_video_file_stem.to_os_string();
                output_file_stem.push("_osd");
                Path::new(&output_file_stem).with_extension("webm")
            },
            (None, None) => {
                let osd_file = common_args.osd_file();
                let mut output_file_stem = Path::new(osd_file.file_stem().ok_or_else(|| anyhow!("OSD file has no file name"))?).as_os_str().to_os_string();
                output_file_stem.push("_osd");
                osd_file.with_file_name(output_file_stem).with_extension("webm")
            }
        };
        let mut overlay_generator = generate_overlay_prepare_generator(common_args)?;
        overlay_generator.generate_overlay_video(*codec, common_args.start_end().start(), common_args.start_end().end(), output_video_path, common_args.frame_shift()?, *overwrite).await?;
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

async fn fix_video_audio_command<P: AsRef<Path>, Q: AsRef<Path>>(input_video_file: P, output_video_file: &Option<Q>, overwrite: bool, sync: bool, volume: bool) -> anyhow::Result<()> {
    let fix_type = match (sync, volume) {
        (true, true) | (false, false) => VideoAudioFixType::SyncAndVolume,
        (true, false) => VideoAudioFixType::Sync,
        (false, true) => VideoAudioFixType::Volume,
    };
    video::fix_dji_air_unit_audio(input_video_file, output_video_file, overwrite, fix_type).await?;
    Ok(())
}

fn current_exe_name() -> anyhow::Result<String> {
    let current_exe = current_exe().map_err(|error| anyhow!("failed to get exe name: {error}"))?;
    Ok(current_exe.file_name().unwrap().to_str().ok_or_else(|| anyhow!("exe file name contains invalid UTF-8 characters"))?.to_string())
}

fn generate_shell_autocompletion_files_command(arg: &GenerateShellAutoCompletionFilesArg) -> anyhow::Result<()> {
    let current_exe_name = current_exe_name()?;
    match arg {
        GenerateShellAutoCompletionFilesArg::All =>
            for shell in Shell::iter() {
                shell.generate_completion_file(&current_exe_name)?;
            },
        GenerateShellAutoCompletionFilesArg::Shell(shell) =>
            shell.generate_completion_file(&current_exe_name)?,
    }
    Ok(())
}

fn generate_man_pages_command() -> anyhow::Result<()> {
    let current_exe_name = current_exe_name()?;
    generate_exe_man_page(&current_exe_name)?;
    generate_man_page_for_subcommands(&current_exe_name)?;
    Ok(())
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    env_logger::builder()
        .format(|buf, record| {
            let level_style = buf.default_level_style(record.level());
            write!(buf, "{:<5}", level_style.value(record.level()))?;
            let mut style = buf.style();
            style.set_color(Color::White).set_bold(true);
            write!(buf, "{}", style.value(" > "))?;
            writeln!(buf, "{}", record.args())
        })
        .parse_filters(cli.log_level().to_string().as_str())
        .init();

    let command_result = match &cli.command {

        command @ Commands::GenerateOverlayFrames {..} => generate_overlay_frames_command(command),
        command @ Commands::GenerateOverlayVideo {..} => generate_overlay_video_command(command).await,
        command @ Commands::TranscodeVideo {..} => transcode_video_command(command).await,
        Commands::DisplayOSDFileInfo { osd_file } => display_osd_file_info_command(osd_file),

        Commands::CutVideo { start_end, input_video_file, output_video_file, overwrite } =>
            video::cut(input_video_file, output_video_file, *overwrite, start_end).await.map_err(anyhow::Error::new),

        Commands::FixVideoAudio { input_video_file, output_video_file, overwrite, sync, volume } =>
            fix_video_audio_command(input_video_file, output_video_file, *overwrite, *sync, *volume).await,

        Commands::PlayVideoWithOSD { video_file, osd_video_file } =>
            video::play_with_osd(video_file, osd_video_file).map_err(anyhow::Error::new),

        Commands::GenerateShellAutocompletionFiles { shell } => generate_shell_autocompletion_files_command(shell),

        Commands::GenerateManPages => generate_man_pages_command(),
    };

    if let Err(error) = command_result {
        log::error!("{}", error);
        exit(1);
    }
}
