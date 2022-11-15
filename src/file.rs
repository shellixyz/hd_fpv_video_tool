
use std::{io::Error as IOError, path::{PathBuf, Path}, fmt::Display, fs::File};

use thiserror::Error;
use getset::Getters;


#[derive(Debug)]
pub enum Action {
    Close,
    Create,
    Open,
    Read,
    Seek,
    Write,
}

impl Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Action::*;
        let action_str = match self {
            Close => "closing",
            Create => "creating",
            Open => "opening",
            Read => "reading",
            Seek => "seeking",
            Write => "writing",
        };
        f.write_str(action_str)
    }
}

#[derive(Debug, Error, Getters)]
#[getset(get = "pub")]
#[error("error {action} {path}: {error}")]
pub struct Error {
    action: Action,
    path: PathBuf,
    error: IOError,
}

impl Error {
    pub fn new<P: AsRef<Path>>(action: Action, path: P, error: IOError) -> Self {
        Self { action, path: path.as_ref().to_path_buf(), error }
    }
}

// impl Display for Error {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{} {}: {}", self.action, self.path.to_string_lossy(), self.error)
//     }
// }

pub fn open<P: AsRef<Path>>(path: P) -> Result<File, Error> {
    std::fs::File::open(&path).map_err(|error| Error::new(Action::Open, path, error))
}

pub fn create<P: AsRef<Path>>(path: P) -> Result<File, Error> {
    std::fs::File::create(&path).map_err(|error| Error::new(Action::Create, path, error))
}

#[derive(Debug, Error, Getters)]
#[getset(get = "pub")]
#[error("error hard linking {original_path} -> {link_path}: {error}")]
pub struct HardLinkError {
    original_path: PathBuf,
    link_path: PathBuf,
    error: IOError,
}

impl HardLinkError {
    pub fn new<P: AsRef<Path>, Q: AsRef<Path>>(original_path: P, link_path: Q, error: IOError) -> Self {
        Self { original_path: original_path.as_ref().to_path_buf(), link_path: link_path.as_ref().to_path_buf(), error }
    }
}

pub fn hard_link<P: AsRef<Path>, Q: AsRef<Path>>(original_path: P, link_path: Q) -> Result<(), HardLinkError> {
    std::fs::hard_link(&original_path, &link_path).map_err(|error| HardLinkError::new(original_path, link_path, error))
}