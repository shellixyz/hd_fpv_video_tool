
use std::{
    fmt::Display,
    io::SeekFrom,
    path::{
        Path,
        PathBuf,
    },
};

use byte_struct::*;

use getset::{Getters, CopyGetters};
use derive_more::From;
use strum::Display;
use thiserror::Error;

use hd_fpv_osd_font_tool::prelude::*;

pub mod frame;
pub mod tile_indices;
pub mod sorted_frames;

use crate::{
    osd::{
        dji::InvalidDimensionsError,
    },
    file::{
        Error as FileError,
        FileWithPath,
    },
};

use super::{
    Dimensions,
    Kind,
};

use self::{
    frame::{
        Frame,
        Header as FrameHeader,
    },
    tile_indices::{
        TileIndex,
        TileIndices,
    }, sorted_frames::{SortedUniqFrames},
};


const SIGNATURE: &str = "MSPOSD\x00";

#[derive(Debug, Error, From)]
pub enum OpenError {
    #[error(transparent)]
    FileError(FileError),
    #[error("invalid DJI OSD file header in file {file_path}")]
    InvalidSignature { file_path: PathBuf },
    #[error("invalid OSD dimensions in file {file_path}: {dimensions}")]
    InvalidOSDDimensions { file_path: PathBuf, dimensions: Dimensions }
}

impl OpenError {

    fn invalid_signature<P: AsRef<Path>>(file_path: P) -> Self {
        Self::InvalidSignature { file_path: file_path.as_ref().to_path_buf() }
    }

    fn invalid_osd_dimensions<P: AsRef<Path>>(file_path: P, dimensions: Dimensions) -> Self {
        Self::InvalidOSDDimensions { file_path: file_path.as_ref().to_path_buf(), dimensions }
    }

}

#[derive(Debug, Error, From)]
pub enum ReadError {
    #[error(transparent)]
    FileError(FileError),
    #[error("Unexpected end of file: {file_path}")]
    UnexpectedEOF { file_path: PathBuf }
}

impl ReadError {
    fn unexpected_eof<P: AsRef<Path>>(file_path: P) -> Self {
        Self::UnexpectedEOF { file_path: file_path.as_ref().to_path_buf() }
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

#[derive(Debug, Display, Clone, Copy)]
pub enum FontVariant {
    Generic,
    Ardupilot,
    Betaflight,
    INAV,
    KISSUltra,
    Unknown
}

impl FontVariant {
    pub fn font_set_ident(&self) -> Option<&str> {
        use FontVariant::*;
        match self {
            Ardupilot => Some("ardu"),
            INAV => Some("inav"),
            Betaflight => Some("bf"),
            KISSUltra => Some("ultra"),
            Generic | Unknown => None,
        }
    }
}

impl From<u8> for FontVariant {
    fn from(value: u8) -> Self {
        use FontVariant::*;
        match value {
            0 => Generic,
            1 => Betaflight,
            2 => INAV,
            3 => Ardupilot,
            4 => KISSUltra,
            _ => Unknown,
        }
    }
}

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
        FontVariant::from(self.font_variant_id)
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

const FIRST_FRAME_FILE_POS: u64 = (SIGNATURE.len() + FileHeaderRaw::BYTE_LEN) as u64;

#[derive(Getters, CopyGetters)]
pub struct Reader {
    file: FileWithPath,
    #[getset(get = "pub")]
    header: FileHeader,
    #[getset(get_copy = "pub")]
    osd_kind: Kind
}

impl Reader {

    fn check_signature<P: AsRef<Path>>(file_path: P, file: &mut FileWithPath) -> Result<(), OpenError> {
        let mut signature = [0; SIGNATURE.len()];
        file.read_exact(&mut signature)?;
        if signature != SIGNATURE.as_bytes() {
            return Err(OpenError::invalid_signature(&file_path))
        }
        Ok(())
    }

    fn read_header(file: &mut FileWithPath) -> Result<FileHeaderRaw, OpenError> {
        let mut header_bytes = [0; FileHeaderRaw::BYTE_LEN];
        file.read_exact(&mut header_bytes)?;
        let header = FileHeaderRaw::read_bytes(&header_bytes);
        Ok(header)
    }

    pub fn open<P: AsRef<Path>>(file_path: P) -> Result<Self, OpenError> {
        let mut file = FileWithPath::open(&file_path)?;
        Self::check_signature(&file_path,&mut file)?;
        let header: FileHeader = Self::read_header(&mut file).unwrap().into();
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

    pub fn read_frame(&mut self) -> Result<Option<Frame>, ReadError> {
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

    pub fn frames(&mut self) -> Result<SortedUniqFrames, ReadError> {
        self.rewind()?;
        let osd_kind = self.osd_kind;
        let font_variant = self.header.font_variant();
        let mut frames = vec![];
        for frame_read_result in self {
            match frame_read_result {
                Ok(frame) => frames.push(frame),
                Err(error) => return Err(error),
            }
        }
        Ok(SortedUniqFrames::new(osd_kind, font_variant, frames))
    }

    pub fn rewind(&mut self) -> Result<(), FileError> {
        self.file.seek(SeekFrom::Start(FIRST_FRAME_FILE_POS))?;
        Ok(())
    }

    fn keep_position_do<F, X, E>(&mut self, f: F) -> Result<X, E>
    where F: FnOnce(&mut Self) -> Result<X, E>
    {
        let starting_position = self.file.pos();
        let return_value = f(self);
        self.file.seek(SeekFrom::Start(starting_position)).unwrap();
        return_value
    }

    pub fn last_frame_frame_index(&mut self) -> Result<u32, ReadError> {
        self.keep_position_do(|reader| {
            reader.rewind()?;
            Ok(reader.frames()?.last().unwrap().index())
        })
    }

    pub fn max_used_tile_index(&mut self) -> Result<TileIndex, ReadError> {
        self.keep_position_do(|reader| {
            reader.rewind()?;
            Ok(*reader.frames()?.iter().flat_map(|frame| frame.tile_indices().as_slice()).max().unwrap())
        })
    }

    pub fn iter(&mut self) -> Iter {
        self.into_iter()
    }

    // pub fn into_frame_overlay_generator(self, font_dir: &FontDir, font_ident: &Option<Option<&str>>, scale: Scaling) -> Result<FrameOverlayGenerator, DrawFrameOverlayError> {
    //     FrameOverlayGenerator::new(self, font_dir, font_ident, scale)
    // }

    // pub fn into_video_frames_iter(mut self, first_frame: u32, last_frame: Option<u32>, frame_shift: i32) -> Result<VideoFramesIntoIter, ReadError> {

    //     let frames = self.frames()?;

    //     let first_video_frame_index = first_frame as i32 - frame_shift;
    //     let first_frame_index = frames.iter().position(|frame| (frame.index() as i32) >= first_video_frame_index);
    //     let osd_file_frames = match first_frame_index {
    //         Some(index) => frames[index..].to_vec(),
    //         None => vec![],
    //     };

    //     Ok(VideoFramesIntoIter {
    //         frames: osd_file_frames,
    //         frame_index: 0,
    //         video_frame_index: first_frame,
    //         last_video_frame_index: last_frame,
    //         video_frame_shift: frame_shift,
    //     })
    // }

    // pub fn into_video_frames_iter(self, first_frame: u32, last_frame: Option<u32>, frame_shift: i32) -> Result<VideoFramesIter, ReadError> {
    //     Ok(self.frames()?.video_frames_iter(first_frame, last_frame, frame_shift))
    // }

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

// pub struct VideoFramesIntoIter {
//     frames: Vec<Frame>,
//     frame_index: usize,
//     video_frame_index: u32,
//     last_video_frame_index: Option<u32>,
//     video_frame_shift: i32,
// }

// impl Iterator for VideoFramesIntoIter {
//     type Item = Option<Frame>;

//     fn next(&mut self) -> Option<Self::Item> {
//         match self.last_video_frame_index {
//             Some(last_frame) => {
//                 if self.video_frame_index > last_frame {
//                     return None;
//                 } else if self.frame_index >= self.frames.len() {
//                     self.video_frame_index += 1;
//                     return Some(None);
//                 }
//             },
//             None => {
//                 if self.frame_index >= self.frames.len() {
//                     return None;
//                 }
//             }
//         }

//         let current_frame = &self.frames[self.frame_index];
//         let actual_frame_video_frame_index = current_frame.index() as i32 + self.video_frame_shift;

//         let frame =
//             if (self.video_frame_index as i32) < actual_frame_video_frame_index {
//                 None
//             } else {
//                 self.frame_index += 1;
//                 Some(current_frame.clone())
//             };

//         self.video_frame_index += 1;

//         Some(frame)
//     }

// }