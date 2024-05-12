
use std::{
    io::{
        Error as IOError,
        Write,
    },
    path::PathBuf,
};

use clap::{ValueEnum, CommandFactory};
use strum::EnumIter;
use clap_complete::generate as clap_complete_generate;
use fs_err::File;

use super::cli::Cli;


const SHELL_COMPLETION_FILES_DIR: &str = "shell_completions";


#[derive(Debug, Clone)]
pub enum GenerateShellAutoCompletionFilesArg {
    All,
    Shell(Shell)
}

pub fn generate_shell_autocompletion_files_arg_parser(value: &str) -> Result<GenerateShellAutoCompletionFilesArg, String> {
    match value {
        "all" => Ok(GenerateShellAutoCompletionFilesArg::All),
        _ => Ok(GenerateShellAutoCompletionFilesArg::Shell(Shell::from_str(value, true)?))
    }
}

macro_rules! shell_enum_and_impl {
    ($($shell:ident),+) => {

        #[derive(Debug, Clone, ValueEnum, EnumIter, strum::Display)]
        #[allow(clippy::enum_variant_names)]
        pub enum Shell {
            $($shell),+
        }

        impl Shell {
            pub fn generate_completion_file(&self, current_exe_name: &str) -> Result<(), IOError> {
                use Shell::*;
                let mut file = File::create(self.completion_file_path(current_exe_name))?;
                let mut buffer: Vec<u8> = Default::default();
                match self {
                    $($shell => clap_complete_generate(clap_complete::shells::$shell, &mut Cli::command(), current_exe_name, &mut buffer),)+
                }
                file.write_all(&buffer)?;
                Ok(())
            }

            pub fn completion_file_path(&self, current_exe_name: &str) -> PathBuf {
                [PathBuf::from(SHELL_COMPLETION_FILES_DIR), PathBuf::from(current_exe_name).with_extension(self.to_string())].iter().collect()
            }
        }

    };
}

shell_enum_and_impl!(Bash, Elvish, Fish, PowerShell, Zsh);