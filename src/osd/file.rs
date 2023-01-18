

use std::{io::Error as IOError, path::{PathBuf, Path}};

use derive_more::From;
use thiserror::Error;

pub mod frame;
pub mod sorted_frames;

pub use frame::Frame;

pub use self::sorted_frames::SortedUniqFrames;

use super::tile_indices::TileIndex;

#[derive(Debug, Error, From)]
pub enum ReadError {
    #[error(transparent)]
    FileError(IOError),
    #[error("Unexpected end of file: {file_path}")]
    UnexpectedEOF { file_path: PathBuf }
}

impl ReadError {
    pub fn unexpected_eof<P: AsRef<Path>>(file_path: P) -> Self {
        Self::UnexpectedEOF { file_path: file_path.as_ref().to_path_buf() }
    }
}

trait FileReader {
    fn read_frame(&mut self) -> Result<Option<Frame>, ReadError>;
    fn frames(&mut self) -> Result<SortedUniqFrames, ReadError>;
    fn last_frame_frame_index(&mut self) -> Result<u32, ReadError>;
    fn max_used_tile_index(&mut self) -> Result<TileIndex, ReadError>;
}