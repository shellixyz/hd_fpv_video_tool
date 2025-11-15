use std::{
	io::Error as IOError,
	ops::Deref,
	path::{Path, PathBuf},
};

// use derive_more::{Error, From};
use image::{DynamicImage, EncodableLayout, ImageBuffer, ImageError, PixelWithColorType, io::Reader as ImageReader};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReadError {
	#[error("failed opening image file `{file_path}`: {error}")]
	OpenError { file_path: PathBuf, error: IOError },
	#[error("failed decoding image file `{file_path}`: {error}")]
	DecodeError { file_path: PathBuf, error: ImageError },
}

impl ReadError {
	pub fn open_error<P: AsRef<Path>>(path: P, error: IOError) -> Self {
		Self::OpenError {
			file_path: path.as_ref().to_path_buf(),
			error,
		}
	}

	pub fn decode_error<P: AsRef<Path>>(path: P, error: ImageError) -> Self {
		Self::DecodeError {
			file_path: path.as_ref().to_path_buf(),
			error,
		}
	}
}

/// Reads an image file from the specified path.
///
/// # Errors
/// Returns a `ReadError` if there are issues opening or decoding the image file.
pub fn read_image_file<P: AsRef<Path>>(path: P) -> Result<DynamicImage, ReadError> {
	let reader = ImageReader::open(&path).map_err(|error| ReadError::open_error(&path, error))?;
	reader.decode().map_err(|error| ReadError::decode_error(&path, error))
}

#[derive(Debug, Error)]
#[error("failed to write image file `{file_path}`: {error}")]
pub struct WriteError {
	file_path: PathBuf,
	error: ImageError,
}

impl WriteError {
	pub fn new<P: AsRef<Path>>(path: P, error: ImageError) -> Self {
		Self {
			file_path: path.as_ref().to_path_buf(),
			error,
		}
	}
}

pub trait WriteImageFile {
	/// Writes the image to the specified file path.
	///
	/// # Errors
	/// Returns a `WriteError` if there are issues saving the image file.
	fn write_image_file<Q: AsRef<Path>>(&self, path: Q) -> Result<(), WriteError>;
}

impl<P, Container> WriteImageFile for ImageBuffer<P, Container>
where
	P: PixelWithColorType,
	[P::Subpixel]: EncodableLayout,
	Container: Deref<Target = [P::Subpixel]>,
{
	fn write_image_file<Q: AsRef<Path>>(&self, path: Q) -> Result<(), WriteError> {
		self.save(&path).map_err(|error| WriteError::new(&path, error))
	}
}
