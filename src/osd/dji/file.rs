
use std::{
    fmt::Display,
    io::{
        Error as IOError,
        SeekFrom, Read, Seek,
    },
    path::{
        Path,
        PathBuf,
    }, ops::RangeInclusive,
};

use byte_struct::*;

use getset::{Getters, CopyGetters};
use derive_more::From;
use itertools::Itertools;
use regex::Regex;
use thiserror::Error;
use lazy_static::lazy_static;
use fs_err::File;

use hd_fpv_osd_font_tool::prelude::*;

use crate::{
    osd::{
        Dimensions, FontVariant, file::{ReadError, Frame, sorted_frames::SortedUniqFrames, GenericReader}, Kind, TileIndices, tile_indices::TileIndex, kind::InvalidDimensionsError,
    },
    video::FrameIndex as VideoFrameIndex,
};

const SIGNATURE: &str = "MSPOSD\x00";
const SUPPORTED_FORMAT_VERSIONS: RangeInclusive<u16> = 1..=1;

#[derive(Debug, Error, From)]
pub enum OpenError {
    #[error(transparent)]
    FileError(IOError),
    #[error("invalid DJI OSD file header in file {file_path}")]
    InvalidSignature { file_path: PathBuf },
    #[error("invalid OSD dimensions in OSD file {file_path}: {dimensions}")]
    InvalidOSDDimensions { file_path: PathBuf, dimensions: Dimensions },
    #[error("unsupported OSD file format version: {0}")]
    UnsupportedFileFormatVersion(u16),
}

impl OpenError {

    fn invalid_signature<P: AsRef<Path>>(file_path: P) -> Self {
        Self::InvalidSignature { file_path: file_path.as_ref().to_path_buf() }
    }

    fn invalid_osd_dimensions<P: AsRef<Path>>(file_path: P, dimensions: Dimensions) -> Self {
        Self::InvalidOSDDimensions { file_path: file_path.as_ref().to_path_buf(), dimensions }
    }

}

#[derive(ByteStruct, Debug)]
#[byte_struct_le]
struct FileHeaderRaw {
    format_version: u16,
    width_tiles: u8,
    height_tiles: u8,
    tile_width: u8,
    tile_height: u8,
    x_offset: u16,
    y_offset: u16,
    font_variant: u8
}

#[derive(Debug, Getters)]
#[getset(get = "pub")]
pub struct Offset {
    x: u16,
    y: u16
}

impl Display for Offset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "x: {}, y: {}", self.x, self.y)
    }
}

#[derive(Debug, Error)]
#[error("unknown font variant ID: {0}")]
pub struct UnknownFontVariantID(pub u8);

#[derive(Debug, Getters)]
#[getset(get = "pub")]
pub struct FileHeader {
    format_version: u16,
    osd_dimensions: Dimensions,
    tile_dimensions: TileDimensions,
    offset: Offset,
    font_variant_id: u8
}

impl FileHeader {
    pub fn font_variant(&self) -> FontVariant {
        use FontVariant::*;
        match self.font_variant_id {
            0 => Generic,
            1 => Betaflight,
            2 => INAV,
            3 => Ardupilot,
            4 => KISSUltra,
            _ => Unknown,
        }
    }
}

impl From<FileHeaderRaw> for FileHeader {
    fn from(fhr: FileHeaderRaw) -> Self {
        Self {
            format_version: fhr.format_version,
            osd_dimensions: Dimensions::new(fhr.width_tiles as u32, fhr.height_tiles as u32),
            tile_dimensions: TileDimensions { width: fhr.tile_width as u32, height: fhr.tile_height as u32 },
            offset: Offset { x: fhr.x_offset, y: fhr.y_offset },
            font_variant_id: fhr.font_variant
        }
    }
}

#[derive(ByteStruct, Debug, CopyGetters)]
#[getset(get_copy = "pub")]
#[byte_struct_le]
pub struct FrameHeader {
    frame_index: VideoFrameIndex,
    data_len: u32
}

const FIRST_FRAME_FILE_POS: u64 = (SIGNATURE.len() + FileHeaderRaw::BYTE_LEN) as u64;

#[derive(Getters, CopyGetters)]
pub struct Reader {
    file: File,
    #[getset(get = "pub")]
    header: FileHeader,
    #[getset(get_copy = "pub")]
    osd_kind: Kind
}

impl Reader {

    fn check_signature<P: AsRef<Path>>(file_path: P, file: &mut File) -> Result<(), OpenError> {
        let mut signature = [0; SIGNATURE.len()];
        file.read_exact(&mut signature)?;
        if signature != SIGNATURE.as_bytes() {
            return Err(OpenError::invalid_signature(&file_path))
        }
        Ok(())
    }

    fn read_header(file: &mut File) -> Result<FileHeaderRaw, OpenError> {
        let mut header_bytes = [0; FileHeaderRaw::BYTE_LEN];
        file.read_exact(&mut header_bytes)?;
        let header = FileHeaderRaw::read_bytes(&header_bytes);
        if ! SUPPORTED_FORMAT_VERSIONS.contains(&header.format_version) {
            return Err(OpenError::UnsupportedFileFormatVersion(header.format_version));
        }
        Ok(header)
    }

    pub fn open<P: AsRef<Path>>(file_path: P) -> Result<Self, OpenError> {
        let mut file = File::open(&file_path)?;
        Self::check_signature(&file_path,&mut file)?;
        let header: FileHeader = Self::read_header(&mut file)?.into();
        let osd_kind = Kind::try_from(header.osd_dimensions()).map_err(|error| {
            let InvalidDimensionsError(dimensions) = error;
            OpenError::invalid_osd_dimensions(&file_path, dimensions)
        })?;
        log::info!("detected OSD file with {osd_kind} tile layout");
        Ok(Self { file, header, osd_kind })
    }

    fn read_frame_header(&mut self) -> Result<Option<FrameHeader>, ReadError> {
        let mut frame_header_bytes = [0; FrameHeader::BYTE_LEN];
        match self.file.read(&mut frame_header_bytes)? {
            0 => Ok(None),
            FrameHeader::BYTE_LEN => Ok(Some(FrameHeader::read_bytes(&frame_header_bytes))),
            _ => Err(ReadError::unexpected_eof(self.file.path()))
        }
    }

    // pub fn read_frame(&mut self) -> Result<Option<Frame>, ReadError> {
    //     let header = match self.read_frame_header()? {
    //         Some(header) => header,
    //         None => return Ok(None),
    //     };
    //     let mut data_bytes= vec![0; header.data_len() as usize * 2];
    //     self.file.read_exact(&mut data_bytes)?;
    //     let tile_indices = TileIndices::new(data_bytes.chunks_exact(u16::BYTE_LEN)
    //         .map(|bytes| u16::from_le_bytes(bytes.try_into().unwrap())).collect());
    //     Ok(Some(Frame::new(header.frame_index(), tile_indices)))
    // }

    // pub fn frames(&mut self) -> Result<SortedUniqFrames, ReadError> {
    //     self.rewind()?;
    //     let osd_kind = self.osd_kind;
    //     let font_variant = self.header.font_variant();
    //     let mut frames = vec![];
    //     for frame_read_result in self {
    //         match frame_read_result {
    //             Ok(frame) => frames.push(frame),
    //             Err(error) => return Err(error),
    //         }
    //     }
    //     let frames = frames.into_iter().sorted_unstable_by_key(Frame::index).unique_by(Frame::index).collect();
    //     Ok(SortedUniqFrames::new(osd_kind, font_variant, frames))
    // }

    pub fn rewind(&mut self) -> Result<(), IOError> {
        self.file.seek(SeekFrom::Start(FIRST_FRAME_FILE_POS))?;
        Ok(())
    }

    fn keep_position_do<F, X, E>(&mut self, f: F) -> Result<X, E>
    where F: FnOnce(&mut Self) -> Result<X, E>
    {
        let starting_position = self.file.stream_position().unwrap();
        let return_value = f(self);
        self.file.seek(SeekFrom::Start(starting_position)).unwrap();
        return_value
    }

    // pub fn last_frame_frame_index(&mut self) -> Result<u32, ReadError> {
    //     self.keep_position_do(|reader| {
    //         Ok(reader.frames()?.last().unwrap().index())
    //     })
    // }

    // pub fn max_used_tile_index(&mut self) -> Result<TileIndex, ReadError> {
    //     self.keep_position_do(|reader| {
    //         Ok(*reader.frames()?.iter().flat_map(|frame|
    //             frame.tile_indices().as_slice()
    //         ).max().unwrap())
    //     })
    // }

    pub fn iter(&mut self) -> Iter {
        self.into_iter()
    }

}

impl GenericReader for Reader {
    fn read_frame(&mut self) -> Result<Option<Frame>, ReadError> {
        let header = match self.read_frame_header()? {
            Some(header) => header,
            None => return Ok(None),
        };
        let mut data_bytes= vec![0; header.data_len() as usize * 2];
        self.file.read_exact(&mut data_bytes)?;
        let tile_indices = TileIndices::new(data_bytes.chunks_exact(u16::BYTE_LEN)
            .map(|bytes| u16::from_le_bytes(bytes.try_into().unwrap())).collect());
        Ok(Some(Frame::new(header.frame_index(), tile_indices)))
    }

    fn frames(&mut self) -> Result<SortedUniqFrames, ReadError> {
        self.rewind()?;
        let osd_kind = self.osd_kind;
        let font_variant = self.header.font_variant();
        let mut frames = vec![];
        let osd_dimensions = self.header.osd_dimensions;
        for frame_read_result in self {
            match frame_read_result {
                Ok(frame) => frames.push(frame),
                Err(error) => return Err(error),
            }
        }
        let frames = frames.into_iter().sorted_unstable_by_key(Frame::index).unique_by(Frame::index).collect::<Vec<Frame>>();
        'outer: for frame in frames.iter() {
            for (coordinates, tile_index) in frame.enumerate_tile_indices() {
                if tile_index > 0 && (coordinates.x as u32 >= osd_dimensions.width || coordinates.y as u32 >= osd_dimensions.height) {
                    log::warn!("the OSD dimensions in the OSD file header do not seem to match the actual data in the file, the OSD might not be rendered fully");
                    break 'outer;
                }
            }
        }
        Ok(SortedUniqFrames::new(osd_kind, font_variant, frames))
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
    lazy_static! { static ref DJI_VIDEO_FILE_RE: Regex = Regex::new(r"\A(?:DJI(?:G|U)(\d{4}))").unwrap(); }

    if let Some(captures) = DJI_VIDEO_FILE_RE.captures(&file_stem) {
        let dji_file_number = captures.get(1).unwrap().as_str();
        let osd_file_path = video_file_path.with_file_name(format!("DJIG{dji_file_number}")).with_extension("osd");
        if osd_file_path.is_file() {
            log::info!("found: {}", osd_file_path.to_string_lossy());
            return Some(osd_file_path);
        } else {
            log::info!("not found: {}", osd_file_path.to_string_lossy());
        }
    }

    None
}