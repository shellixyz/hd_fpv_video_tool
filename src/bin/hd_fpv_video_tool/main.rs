#![forbid(unsafe_code)]

use std::{
	env::current_exe,
	io::Write,
	path::{Path, PathBuf},
	process::exit,
};

use clap::Parser;
use env_logger::fmt::Color;
use strum::IntoEnumIterator;

use anyhow::anyhow;

use hd_fpv_video_tool::{cli::generate_overlay_args::GenerateOverlayArgsBuilder, osd::file::GenericReader, prelude::*};
mod cli;
mod man_pages;
mod shell_autocompletion;

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
			println!(
				"OSD Font variant: {} ({})",
				header.font_variant_id(),
				header.font_variant()
			);
		},
		osd::file::Reader::WSA(reader) => {
			let header = reader.header();
			println!("OSD file type: Walksnail Avatar");
			println!(
				"OSD Font variant: {} ({})",
				header.font_variant_id(),
				header.font_variant()
			);
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
			frames => format!("every {frames} frames"),
		};
		let refresh_freq = 60.0 / refresh_interval_frames;
		println!(
			"OSD update rate: {refresh_percent_frames:.0}% of the video frames ({refresh_freq:.1}Hz or approximately {refresh_interval_frames_str})"
		);
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
		common_args.hide_items(),
	)?;
	Ok(overlay_generator)
}

fn generate_overlay_frames_command(command: &Commands) -> anyhow::Result<()> {
	if let Commands::GenerateOverlayFrames {
		common_args,
		output_dir,
	} = command
	{
		common_args.check_valid()?;
		let output_dir = match (output_dir, common_args.target_video_file()) {
			(Some(output_dir), _) => output_dir.clone(),
			(None, Some(target_video_file)) => {
				let target_video_file_stem = target_video_file
					.file_stem()
					.ok_or_else(|| anyhow!("target video file has no file name"))?;
				let mut output_file_stem = target_video_file_stem.to_os_string();
				output_file_stem.push("_osd_frames");
				PathBuf::from(output_file_stem)
			},
			(None, None) => {
				let osd_file = common_args.osd_file();
				let mut output_dir_name = Path::new(
					osd_file
						.file_stem()
						.ok_or_else(|| anyhow!("OSD file has no file name"))?,
				)
				.as_os_str()
				.to_os_string();
				output_dir_name.push("_osd_frames");
				osd_file.with_file_name(output_dir_name)
			},
		};
		let mut overlay_generator = generate_overlay_prepare_generator(common_args)?;
		overlay_generator.save_frames_to_dir(
			common_args.start_end().start(),
			common_args.start_end().end(),
			output_dir,
			common_args.frame_shift()?,
		)?;
	}
	Ok(())
}

fn overlay_video_file_name_from_target_video_file_name(target_video_file: impl AsRef<Path>) -> anyhow::Result<PathBuf> {
	let target_video_file_stem = target_video_file
		.as_ref()
		.file_stem()
		.ok_or_else(|| anyhow!("target video file has no file name"))?;
	let mut output_file_stem = target_video_file_stem.to_os_string();
	output_file_stem.push("_osd");
	Ok(Path::new(&output_file_stem).with_extension("webm"))
}

async fn generate_overlay_video_command(command: &Commands) -> anyhow::Result<()> {
	if let Commands::GenerateOverlayVideo {
		common_args,
		video_file,
		overwrite,
		codec,
	} = command
	{
		common_args.check_valid()?;
		let output_video_path = match (video_file, common_args.target_video_file()) {
			(Some(output_video_file), _) => output_video_file.clone(),
			(None, Some(target_video_file)) => {
				// let target_video_file_stem = target_video_file
				// 	.file_stem()
				// 	.ok_or_else(|| anyhow!("target video file has no file name"))?;
				// let mut output_file_stem = target_video_file_stem.to_os_string();
				// output_file_stem.push("_osd");
				// Path::new(&output_file_stem).with_extension("webm")
				overlay_video_file_name_from_target_video_file_name(target_video_file)?
			},
			(None, None) => {
				let osd_file = common_args.osd_file();
				let mut output_file_stem = Path::new(
					osd_file
						.file_stem()
						.ok_or_else(|| anyhow!("OSD file has no file name"))?,
				)
				.as_os_str()
				.to_os_string();
				output_file_stem.push("_osd");
				osd_file.with_file_name(output_file_stem).with_extension("webm")
			},
		};
		let mut overlay_generator = generate_overlay_prepare_generator(common_args)?;
		overlay_generator
			.generate_overlay_video(
				*codec,
				common_args.start_end().start(),
				common_args.start_end().end(),
				output_video_path,
				common_args.frame_shift()?,
				*overwrite,
			)
			.await?;
	}
	Ok(())
}

async fn transcode_video_command(command: &Commands) -> anyhow::Result<()> {
	if let Commands::TranscodeVideo {
		osd_args,
		transcode_args,
	} = command
	{
		transcode_args.start_end().check_valid()?;

		match osd_args.osd_file_path(transcode_args.input_video_file())? {
			Some(osd_file_path) if osd_args.osd_overlay_video() => {
				let transcode_output_video_file = video::transcode(transcode_args).await?;
				let osd_overlay_video_file_name = match osd_args.osd_overlay_video_file() {
					Some(osd_overlay_video_file_name) => {
						if !matches!(osd_overlay_video_file_name.extension(), Some(extension) if extension == "webm") {
							return Err(anyhow!("OSD overlay video file name should have the .webm extension"));
						}
						osd_overlay_video_file_name.clone()
					},
					None => overlay_video_file_name_from_target_video_file_name(transcode_output_video_file.clone())?,
				};
				let gov_command = Commands::GenerateOverlayVideo {
					common_args: GenerateOverlayArgsBuilder::default()
						.target_video_file(Some(transcode_output_video_file))
						.hide_regions(osd_args.osd_hide_regions().clone())
						.hide_items(osd_args.osd_hide_items().clone())
						.start_end(transcode_args.start_end().clone())
						.scaling_args(osd_args.osd_scaling_args().into())
						.font_options(osd_args.osd_font_options().into())
						.frame_shift(osd_args.osd_frame_shift())
						.osd_file(osd_file_path)
						.build()
						.unwrap(),
					codec: osd_args.osd_overlay_video_codec(),
					video_file: Some(osd_overlay_video_file_name),
					overwrite: transcode_args.overwrite(),
				};
				generate_overlay_video_command(&gov_command).await?;
			},
			Some(osd_file_path) => video::transcode_burn_osd(transcode_args, osd_file_path, osd_args).await?,
			None => {
				video::transcode(transcode_args).await?;
			},
		}
	}
	Ok(())
}

async fn add_audio_stream_command(command: &Commands) -> anyhow::Result<()> {
	if let Commands::AddAudioStream {
		audio_encoder,
		audio_bitrate,
		input_video_file,
		output_video_file,
		overwrite,
	} = command
	{
		let output_video_file = match output_video_file {
			Some(output_video_file) => output_video_file.clone(),
			None => {
				let mut output_file_stem = Path::new(
					input_video_file
						.file_stem()
						.ok_or_else(|| anyhow!("input file has no file name"))?,
				)
				.as_os_str()
				.to_os_string();
				output_file_stem.push("_with_audio");
				let input_file_extension = input_video_file
					.extension()
					.ok_or_else(|| anyhow!("input file has no extension"))?;
				input_video_file
					.with_file_name(output_file_stem)
					.with_extension(input_file_extension)
			},
		};
		video::add_audio_stream(
			input_video_file,
			output_video_file,
			*overwrite,
			audio_encoder,
			audio_bitrate,
		)
		.await?;
	}
	Ok(())
}

async fn fix_video_audio_command<P: AsRef<Path>, Q: AsRef<Path>>(
	input_video_file: P,
	output_video_file: &Option<Q>,
	overwrite: bool,
	sync: bool,
	volume: bool,
) -> anyhow::Result<()> {
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
	Ok(current_exe
		.file_name()
		.unwrap()
		.to_str()
		.ok_or_else(|| anyhow!("exe file name contains invalid UTF-8 characters"))?
		.to_string())
}

fn generate_shell_autocompletion_files_command(arg: &GenerateShellAutoCompletionFilesArg) -> anyhow::Result<()> {
	let current_exe_name = current_exe_name()?;
	let shell_completion_files_path = Path::new(SHELL_COMPLETION_FILES_DIR);
	if shell_completion_files_path.exists() {
		if shell_completion_files_path.is_dir() {
			return Err(anyhow!("{} is not a directory", SHELL_COMPLETION_FILES_DIR));
		}
	} else {
		std::fs::create_dir(SHELL_COMPLETION_FILES_DIR)?;
	}
	match arg {
		GenerateShellAutoCompletionFilesArg::All => {
			for shell in Shell::iter() {
				shell.generate_completion_file(&current_exe_name)?;
			}
		},
		GenerateShellAutoCompletionFilesArg::Shell(shell) => shell.generate_completion_file(&current_exe_name)?,
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
		command @ Commands::GenerateOverlayFrames { .. } => generate_overlay_frames_command(command),
		command @ Commands::GenerateOverlayVideo { .. } => generate_overlay_video_command(command).await,
		command @ Commands::TranscodeVideo { .. } => transcode_video_command(command).await,
		command @ Commands::AddAudioStream { .. } => add_audio_stream_command(command).await,
		Commands::DisplayOSDFileInfo { osd_file } => display_osd_file_info_command(osd_file),
		Commands::CutVideo {
			start_end,
			input_video_file,
			output_video_file,
			overwrite,
		} => video::cut(input_video_file, output_video_file, *overwrite, start_end)
			.await
			.map_err(anyhow::Error::new),
		Commands::FixVideoAudio {
			input_video_file,
			output_video_file,
			overwrite,
			sync,
			volume,
		} => fix_video_audio_command(input_video_file, output_video_file, *overwrite, *sync, *volume).await,
		Commands::PlayVideoWithOSD {
			video_file,
			osd_video_file,
		} => video::play_with_osd(video_file, osd_video_file).map_err(anyhow::Error::new),
		Commands::SpliceVideos {
			input_video_files,
			output,
			overwrite,
		} => video::splice(input_video_files, output, *overwrite)
			.await
			.map_err(anyhow::Error::new),
		Commands::GenerateShellAutocompletionFiles { shell } => generate_shell_autocompletion_files_command(shell),
		Commands::GenerateManPages => generate_man_pages_command(),
	};

	if let Err(error) = command_result {
		log::error!("{}", error);
		exit(1);
	}
}
