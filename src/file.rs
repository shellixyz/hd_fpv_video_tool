use std::{
	io::Error as IOError,
	mem,
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

/// A RAII guard that deletes an output file when dropped, unless [`defuse`](Self::defuse) is
/// called first.
///
/// Use this to ensure partial / broken output files are cleaned up if an operation fails
/// mid-way through.
///
/// # Usage
/// ```rust,no_run
/// use hd_fpv_video_tool::file::{OutputFileGuard, touch};
///
/// # async fn example() -> anyhow::Result<()> {
/// touch("output.mp4")?;
/// let guard = OutputFileGuard::new("output.mp4");
/// // ... do work that may fail ...
/// guard.defuse(); // call on success — prevents deletion
/// //
/// # Ok(())
/// # }
/// ```
pub struct OutputFileGuard {
	path: Option<PathBuf>,
}

impl OutputFileGuard {
	/// Create a guard for the given path. The file will be removed when this guard is
	/// dropped, unless [`defuse`](Self::defuse) is called first.
	#[must_use]
	pub fn new(path: impl Into<PathBuf>) -> Self {
		Self {
			path: Some(path.into()),
		}
	}

	/// Defuse the guard — the output file will **not** be deleted when this guard drops.
	/// Call this after a successful operation.
	pub fn defuse(mut self) {
		self.path = None;
		mem::forget(self);
	}
}

impl Drop for OutputFileGuard {
	fn drop(&mut self) {
		if let Some(ref path) = self.path
			&& path.exists()
		{
			if let Err(e) = std::fs::remove_file(path) {
				log::warn!("failed to remove partial output file {}: {e}", path.display());
			} else {
				log::info!("removed partial output file: {}", path.display());
			}
		}
	}
}
