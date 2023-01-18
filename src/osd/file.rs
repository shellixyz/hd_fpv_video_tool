

use std::{io::Error as IOError, path::{PathBuf, Path}};

use derive_more::From;
use thiserror::Error;

pub mod frame;
pub mod sorted_frames;

pub use frame::Frame;

pub use self::sorted_frames::SortedUniqFrames;

use super::{tile_indices::TileIndex, FontVariant};

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

pub trait GenericReader {
    fn read_frame(&mut self) -> Result<Option<Frame>, ReadError>;
    fn frames(&mut self) -> Result<SortedUniqFrames, ReadError>;
    fn last_frame_frame_index(&mut self) -> Result<u32, ReadError>;
    fn max_used_tile_index(&mut self) -> Result<TileIndex, ReadError>;
    fn font_variant(&self) -> FontVariant;
}

pub fn find_associated_to_video_file<P: AsRef<Path>>(video_file_path: P) -> Option<PathBuf> {
    let video_file_path = video_file_path.as_ref();
    log::info!("looking for OSD file associated to video file: {}", video_file_path.to_string_lossy());

    let osd_file_path = video_file_path.with_extension("osd");
    if osd_file_path.is_file() {
        log::info!("found: {}", osd_file_path.to_string_lossy());
        return Some(osd_file_path);
    } else {
        log::info!("not found: {}", osd_file_path.to_string_lossy());
    }

    let file_stem = video_file_path.file_stem()?.to_string_lossy();

    if file_stem.starts_with("DJI") {
        super::dji::file::find_associated_to_video_file(video_file_path)
    } else if file_stem.starts_with("Avatar") {
        super::wsa::file::find_associated_to_video_file(video_file_path)
    } else {
        None
    }
}

pub enum Reader {
    DJI(super::dji::file::Reader),
    WSA(super::wsa::file::Reader),
}

// impl Reader {
//     pub fn frames(&mut self) -> Result<SortedUniqFrames, ReadError> {
//         match self {
//             Reader::DJI(reader) => reader.frames(),
//             Reader::WSA(reader) => reader.frames(),
//         }
//     }
// }

impl GenericReader for Reader {
    fn read_frame(&mut self) -> Result<Option<Frame>, ReadError> {
        match self {
            Reader::DJI(reader) => reader.read_frame(),
            Reader::WSA(reader) => reader.read_frame(),
        }
    }

    fn frames(&mut self) -> Result<SortedUniqFrames, ReadError> {
        match self {
            Reader::DJI(reader) => reader.frames(),
            Reader::WSA(reader) => reader.frames(),
        }
    }

    fn last_frame_frame_index(&mut self) -> Result<u32, ReadError> {
        match self {
            Reader::DJI(reader) => reader.last_frame_frame_index(),
            Reader::WSA(reader) => reader.last_frame_frame_index(),
        }
    }

    fn max_used_tile_index(&mut self) -> Result<TileIndex, ReadError> {
        match self {
            Reader::DJI(reader) => reader.max_used_tile_index(),
            Reader::WSA(reader) => reader.max_used_tile_index(),
        }
    }

    fn font_variant(&self) -> FontVariant {
        match self {
            Reader::DJI(reader) => reader.font_variant(),
            Reader::WSA(reader) => reader.font_variant(),
        }
    }
}

#[derive(Debug, Error)]
#[error("unrecognized OSD file: {0}")]
pub struct UnrecognizedOSDFile(PathBuf);

pub fn open(path: impl AsRef<Path>) -> Result<Reader, UnrecognizedOSDFile> {
    let path = path.as_ref();
    if let Some(file_stem) = path.file_stem() {
        let file_stem = file_stem.to_string_lossy();
        if file_stem.starts_with("DJIG") {
            if let Ok(reader) = super::dji::file::Reader::open(&path) {
                return Ok(Reader::DJI(reader));
            }
        } else if file_stem.starts_with("AvatarG") {
            if let Ok(reader) = super::wsa::file::Reader::open(&path) {
                return Ok(Reader::WSA(reader));
            }
        }
    }

    if let Ok(reader) = super::dji::file::Reader::open(&path) {
        return Ok(Reader::DJI(reader));
    }

    if let Ok(reader) = super::wsa::file::Reader::open(&path) {
        return Ok(Reader::WSA(reader));
    }

    Err(UnrecognizedOSDFile(path.to_owned()))
}