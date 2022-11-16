
use std::{
    fmt::Display,
    path::{
        Path,
        PathBuf
    },
    io::Error as IOError,
    ops::Deref
};

use derive_more::{Error, From};
use image::{
    DynamicImage,
    ImageError,
    EncodableLayout,
    ImageBuffer,
    PixelWithColorType,
    io::Reader as ImageReader
};

use crate::file::{
    Error as FileError,
    Action as FileAction
};


#[derive(Debug, Error, From)]
pub enum ReadError {
    OpenError(FileError),
    DecodeError {
        file_path: PathBuf,
        error: ImageError
    }
}

impl ReadError {
    pub fn open_error<P: AsRef<Path>>(path: P, error: IOError) -> Self {
        Self::OpenError(FileError::new(FileAction::Open, path, error))
    }

    pub fn decode_error<P: AsRef<Path>>(path: P, error: ImageError) -> Self {
        Self::DecodeError { file_path: path.as_ref().to_path_buf(), error }
    }
}

impl Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ReadError::*;
        match self {
            OpenError(error) => error.fmt(f),
            DecodeError { file_path, error } => write!(f, "failed decoding {}: #{error}", file_path.to_string_lossy()),
        }
    }
}

pub fn read_image_file<P: AsRef<Path>>(path: P) -> Result<DynamicImage, ReadError> {
    let reader = ImageReader::open(&path) .map_err(|error| ReadError::open_error(&path, error))?;
    reader.decode().map_err(|error| ReadError::decode_error(&path, error) )
}

#[derive(Debug, From, Error)]
pub struct WriteError {
    file_path: PathBuf,
    error: ImageError,
}

impl WriteError {
    pub fn new<P: AsRef<Path>>(path: P, error: ImageError) -> Self {
        Self { file_path: path.as_ref().to_path_buf(), error }
    }
}

impl Display for WriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "failed to write image {}: {}", self.file_path.to_string_lossy(), self.error)
    }
}

pub trait WriteImageFile {
    fn write_image_file<Q: AsRef<Path>>(&self, path: Q) -> Result<(), WriteError>;
}

impl<P, Container> WriteImageFile for ImageBuffer<P, Container>
where
    P: PixelWithColorType,
    [P::Subpixel]: EncodableLayout,
    Container: Deref<Target = [P::Subpixel]>,
{
    fn write_image_file<Q: AsRef<Path>>(&self, path: Q) -> Result<(), WriteError> {
        self.save(&path).map_err(|error| WriteError::new(&path, error) )
    }
}
