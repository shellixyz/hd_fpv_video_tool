use std::{
	io::Error as IOError,
	path::{Path, PathBuf},
};

use fs_err::File;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TouchError {
	#[error("failed creating file {0}: invalid path")]
	InvalidPath(PathBuf),
	#[error("failed creating file {file_path}: directory does not exist: {dir_path}")]
	DirectoryDoesNotExist { file_path: PathBuf, dir_path: PathBuf },
	#[error(transparent)]
	CreateError(#[from] IOError),
}

/// Creates an empty file at the specified path, similar to the Unix `touch` command.
///
/// # Errors
/// Returns a `TouchError` if the file cannot be created, or if the parent directory does not exist.
pub fn touch<P: AsRef<Path>>(path: P) -> Result<(), TouchError> {
	let path = path.as_ref();
	let dir = path
		.parent()
		.ok_or_else(|| TouchError::InvalidPath(path.to_path_buf()))?;
	if !dir.as_os_str().is_empty() && !dir.exists() {
		return Err(TouchError::DirectoryDoesNotExist {
			file_path: path.to_path_buf(),
			dir_path: dir.to_path_buf(),
		});
	}
	File::create(path)?;
	Ok(())
}
