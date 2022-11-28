
#![forbid(unsafe_code)]

use std::{
    process::exit,
    path::Path, env::current_exe
};

use clap::Parser;
use strum::IntoEnumIterator;

use anyhow::anyhow;


use dji_fpv_video_tool::prelude::*;
mod shell_autocompletion;
mod man_pages;
mod cli;

use {cli::*, man_pages::*, shell_autocompletion::*};


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

    pretty_env_logger::formatted_builder().parse_filters(cli.log_level().to_string().as_str()).init();

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
