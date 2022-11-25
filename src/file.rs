
use std::{
    io::{
        Error as IOError,
        SeekFrom,
        Seek,
        Read
    },
    path::{
        PathBuf,
        Path
    },
    fmt::Display,
    fs::File
};

use derive_more::Deref;
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

#[derive(Deref, Getters)]
pub struct FileWithPath {
    #[getset(get = "pub")]
    path: PathBuf,
    #[deref]
    file: File,
}

impl FileWithPath {

    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        Ok(Self {
            path: path.as_ref().to_path_buf(),
            file: File::open(&path).map_err(|error| Error::new(Action::Open, path, error))?
        })
    }

    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        Ok(Self {
            path: path.as_ref().to_path_buf(),
            file: File::create(&path).map_err(|error| Error::new(Action::Create, path, error))?
        })
    }

    pub fn seek(&mut self, pos: SeekFrom) -> Result<u64, Error> {
        self.file.seek(pos).map_err(|error| Error::new(Action::Seek, &self.path, error))
    }

    pub fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Error> {
        self.file.read_exact(buf).map_err(|error| Error::new(Action::Read, &self.path, error))
    }

    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        self.file.read(buf).map_err(|error| Error::new(Action::Read, &self.path, error))
    }

    pub fn pos(&mut self) -> u64 {
        self.file.seek(SeekFrom::Current(0)).unwrap()
    }

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

#[derive(Debug, Error, Getters)]
#[getset(get = "pub")]
#[error("error symlinking {original_path} -> {link_path}: {error}")]
pub struct SymlinkError {
    original_path: PathBuf,
    link_path: PathBuf,
    error: IOError,
}

impl SymlinkError {
    pub fn new<P: AsRef<Path>, Q: AsRef<Path>>(original_path: P, link_path: Q, error: IOError) -> Self {
        Self { original_path: original_path.as_ref().to_path_buf(), link_path: link_path.as_ref().to_path_buf(), error }
    }
}

pub fn symlink<P: AsRef<Path>, Q: AsRef<Path>>(original_path: P, link_path: Q) -> Result<(), SymlinkError> {
    std::os::unix::fs::symlink(&original_path, &link_path).map_err(|error| SymlinkError::new(original_path, link_path, error))
}