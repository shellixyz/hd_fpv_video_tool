
use std::path::PathBuf;
use clap::CommandFactory;

use dji_fpv_video_tool::prelude::*;

use super::cli::Cli;


const MAN_PAGES_DIR: &str = "man_pages";


pub fn command_man_page_path(exe_name: &str, subcommand: Option<&clap::Command>) -> PathBuf {
    let extension = "1";
    let file_name = match subcommand {
        Some(command) => PathBuf::from(format!("{exe_name}-{}", command.get_name())),
        None => PathBuf::from(exe_name),
    };
    [PathBuf::from(MAN_PAGES_DIR), file_name.with_extension(extension)].iter().collect()
}

pub fn generate_exe_man_page(exe_name: &str) -> anyhow::Result<()> {
    let mut file = file::create(command_man_page_path(exe_name, None))?;
    let man = clap_mangen::Man::new(Cli::command());
    let mut buffer: Vec<u8> = Default::default();
    man.render(&mut buffer)?;
    file.write_all(&buffer)?;
    Ok(())
}

pub fn generate_man_page_for_subcommands(exe_name: &str) -> anyhow::Result<()> {
    let command = Cli::command();
    let exclusions = ["generate-shell-autocompletion-files", "generate-man-pages"];
    for subcommand in command.get_subcommands() {
        if ! exclusions.contains(&subcommand.get_name()) {
            let mut file = file::create(command_man_page_path(exe_name, Some(subcommand)))?;
            let mut buffer: Vec<u8> = Default::default();
            let man = clap_mangen::Man::new(subcommand.to_owned());
            man.render(&mut buffer)?;
            file.write_all(&buffer)?;
        }
    }
    Ok(())
}