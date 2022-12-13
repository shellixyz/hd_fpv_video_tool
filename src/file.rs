
use fs_err::File;

use std::{
    io::Error as IOError,
    path::{
        PathBuf,
        Path
    },
};

use thiserror::Error;


#[derive(Debug, Error)]
pub enum CheckWritableError {
    #[error("failed creating file {0}: invalid path")]
    InvalidPath(PathBuf),
    #[error("failed creating file {file_path}: directory does not exist: {dir_path}")]
    DirectoryDoesNotExist {
        file_path: PathBuf,
        dir_path: PathBuf,
    },
    #[error(transparent)]
    CreateError(#[from] IOError),
}

pub fn check_writable<P: AsRef<Path>>(path: P) -> Result<(), CheckWritableError> {
    let path = path.as_ref();
    let dir = path.parent().ok_or_else(|| CheckWritableError::InvalidPath(path.to_path_buf()))?;
    if ! dir.as_os_str().is_empty() && ! dir.exists() {
        return Err(CheckWritableError::DirectoryDoesNotExist {
            file_path: path.to_path_buf(),
            dir_path: dir.to_path_buf()
        })
    }
    File::create(path)?;
    Ok(())
}