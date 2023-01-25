
use std::{
    io::{
        Error as IOError,
        SeekFrom, Read, Seek,
    },
    path::{
        Path,
        PathBuf,
    }, borrow::{Cow, Borrow},
};

use byte_struct::*;

use getset::{Getters, CopyGetters};
use itertools::Itertools;
use regex::Regex;
use thiserror::Error;
use lazy_static::lazy_static;
use fs_err::File;

use crate::{
    osd::{
        Dimensions,
        FontVariant,
        file::{
            ReadError,
            Frame,
            sorted_frames::SortedUniqFrames,
            GenericReader
        },
        Kind,
        TileIndices,
        TileIndex, tile_indices,
    },
    video::FrameIndex as VideoFrameIndex,
};

use super::DIMENSIONS;


#[derive(Debug, Error)]
pub enum OpenError {
    #[error(transparent)]
    FileError(#[from] IOError),
    #[error("invalid WSA OSD file header in {0}")]
    InvalidHeader(PathBuf),
    #[error("WSA OSD file `{0}` has an invalid size")]
    InvalidSize(PathBuf),
}

#[derive(ByteStruct, Debug)]
#[byte_struct_le]
pub struct FileHeaderRaw {
    font_variant_id: [u8; 4],
    unused: [u8; 32],
    width_tiles: u16,
    height_tiles: u16,
}

impl FileHeaderRaw {

    pub fn font_variant_id(&self) -> Cow<str> {
        String::from_utf8_lossy(&self.font_variant_id)
    }

    pub fn font_variant(&self) -> FontVariant {
        use FontVariant::*;
        match self.font_variant_id().borrow() {
            "INAV" => INAV,
            "ARDU" => Ardupilot,
            _ => Unknown,
        }
    }

}

#[derive(Debug, Getters, CopyGetters)]
pub struct FileHeader {
    #[getset(get = "pub")]
    font_variant_id: String,
    #[getset(get_copy = "pub")]
    font_variant: FontVariant,
    #[getset(get_copy = "pub")]
    osd_dimensions: Dimensions,
}

impl From<FileHeaderRaw> for FileHeader {
    fn from(fhr: FileHeaderRaw) -> Self {
        Self {
            font_variant_id: fhr.font_variant_id().to_string(),
            font_variant: fhr.font_variant(),
            osd_dimensions: Dimensions::new(fhr.width_tiles as u32, fhr.height_tiles as u32),
        }
    }
}

#[derive(ByteStruct, Debug, CopyGetters)]
#[getset(get_copy = "pub")]
#[byte_struct_le]
pub struct FrameRaw {
    frame_timestamp: u32, // *100µs
    tile_indices: [[u16; DIMENSIONS.width as usize]; DIMENSIONS.height as usize],
}

impl FrameRaw {
    pub fn frame_index(&self) -> VideoFrameIndex {
        (self.frame_timestamp as f64 * 60.0 / 1_000.0).round() as VideoFrameIndex
    }
}

const FIRST_FRAME_FILE_POS: u64 = FileHeaderRaw::BYTE_LEN as u64;

#[derive(Getters)]
pub struct Reader {
    file: File,
    #[getset(get = "pub")]
    header: FileHeader,
}

impl Reader {

    fn read_header(file: &mut File) -> Result<FileHeaderRaw, OpenError> {
        let mut header_bytes = [0; FileHeaderRaw::BYTE_LEN];
        file.read_exact(&mut header_bytes)?;
        let header = FileHeaderRaw::read_bytes(&header_bytes);
        Ok(header)
    }

    pub fn open<P: AsRef<Path>>(file_path: P) -> Result<Self, OpenError> {
        let mut file = File::open(&file_path)?;
        let header: FileHeader = Self::read_header(&mut file)?.into();
        if header.osd_dimensions != DIMENSIONS {
            return Err(OpenError::InvalidHeader(file_path.as_ref().to_owned()));
        }
        if (file.metadata()?.len() - FileHeaderRaw::BYTE_LEN as u64) % FrameRaw::BYTE_LEN as u64 != 0 {
            return Err(OpenError::InvalidSize(file_path.as_ref().to_owned()));
        }
        Ok(Self { file, header })
    }

    pub fn rewind(&mut self) -> Result<(), IOError> {
        self.file.seek(SeekFrom::Start(FIRST_FRAME_FILE_POS))?;
        Ok(())
    }

    fn keep_position_do<F, X, E>(&mut self, f: F) -> Result<X, E>
    where F: FnOnce(&mut Self) -> Result<X, E>
    {
        let starting_position = self.file.seek(SeekFrom::Current(0)).unwrap();
        let return_value = f(self);
        self.file.seek(SeekFrom::Start(starting_position)).unwrap();
        return_value
    }

    pub fn iter(&mut self) -> Iter {
        self.into_iter()
    }

}

impl GenericReader for Reader {
    fn read_frame(&mut self) -> Result<Option<Frame>, ReadError> {
        let mut frame_raw_bytes = [0; FrameRaw::BYTE_LEN];
        let frame_raw = match self.file.read(&mut frame_raw_bytes)? {
            0 => return Ok(None),
            FrameRaw::BYTE_LEN => FrameRaw::read_bytes(&frame_raw_bytes),
            _ => return Err(ReadError::unexpected_eof(self.file.path()))
        };
        let mut tile_indices = Vec::with_capacity(tile_indices::COUNT);
        let (x_range, y_range) = (0..DIMENSIONS.width as usize, 0..DIMENSIONS.height as usize);
        for x in 0..tile_indices::DIMENSIONS.width as usize {
            for y in 0..tile_indices::DIMENSIONS.height as usize {
                if x_range.contains(&x) && y_range.contains(&y) {
                    tile_indices.push(frame_raw.tile_indices[y][x]);
                } else {
                    tile_indices.push(0);
                }
            }
        }
        Ok(Some(Frame::new(frame_raw.frame_index(), TileIndices::new(tile_indices))))
    }

    fn frames(&mut self) -> Result<SortedUniqFrames, ReadError> {
        self.rewind()?;
        let font_variant = self.header.font_variant();
        let mut frames = vec![];
        for frame_read_result in self {
            match frame_read_result {
                Ok(frame) => frames.push(frame),
                Err(error) => return Err(error),
            }
        }
        let frames = frames.into_iter().sorted_unstable_by_key(Frame::index).unique_by(Frame::index).collect();
        Ok(SortedUniqFrames::new(Kind::WSA, font_variant, frames))
    }

    fn last_frame_frame_index(&mut self) -> Result<u32, ReadError> {
        self.keep_position_do(|reader| {
            Ok(reader.frames()?.last().unwrap().index())
        })
    }

    fn max_used_tile_index(&mut self) -> Result<TileIndex, ReadError> {
        self.keep_position_do(|reader| {
            Ok(*reader.frames()?.iter().flat_map(|frame|
                frame.tile_indices().as_slice()
            ).max().unwrap())
        })
    }

    fn font_variant(&self) -> FontVariant {
        self.header.font_variant()
    }
}

pub struct IntoIter {
    reader: Reader
}

impl Iterator for IntoIter {
    type Item = Result<Frame, ReadError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.reader.read_frame().transpose()
    }
}

impl IntoIterator for Reader {
    type Item = Result<Frame, ReadError>;

    type IntoIter = IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter { reader: self }
    }
}

pub struct Iter<'a> {
    reader: &'a mut Reader
}

impl<'a> Iterator for Iter<'a> {
    type Item = Result<Frame, ReadError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.reader.read_frame().transpose()
    }
}

impl<'a> IntoIterator for &'a mut Reader {
    type Item = Result<Frame, ReadError>;

    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter { reader: self }
    }
}

pub fn find_associated_to_video_file<P: AsRef<Path>>(video_file_path: P) -> Option<PathBuf> {
    let video_file_path = video_file_path.as_ref();
    let file_stem = video_file_path.file_stem()?.to_string_lossy();
    lazy_static! { static ref DJI_VIDEO_FILE_RE: Regex = Regex::new(r"\A(?:Avatar(?:G|S)(\d{4}))").unwrap(); }

    if let Some(captures) = DJI_VIDEO_FILE_RE.captures(&file_stem) {
        let dji_file_number = captures.get(1).unwrap().as_str();
        let osd_file_path = video_file_path.with_file_name(format!("AvatarG{dji_file_number}")).with_extension("osd");
        if osd_file_path.is_file() {
            log::info!("found: {}", osd_file_path.to_string_lossy());
            return Some(osd_file_path);
        } else {
            log::info!("not found: {}", osd_file_path.to_string_lossy());
        }
    }

    None
}