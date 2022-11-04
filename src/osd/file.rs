
use std::error::Error;
use std::fmt::Display;
use std::fs::File;
use std::io::{Error as IOError, Read};
use std::path::Path;

use byte_struct::ByteStruct;
use byte_struct::*;

const SIGNATURE: &str = "MSPOSD\x00";

#[derive(Debug)]
pub enum OpenError {
    IOError(IOError),
    InvalidSignature
}

impl Error for OpenError {}

impl From<IOError> for OpenError {
    fn from(error: IOError) -> Self {
        Self::IOError(error)
    }
}

impl Display for OpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use OpenError::*;
        match self {
            IOError(error) => error.fmt(f),
            InvalidSignature => f.write_str("invalid header"),
        }
    }
}

#[derive(Debug)]
pub enum ReadError {
    IOError(IOError),
    UnexpectedEOF
}

impl Error for ReadError {}

impl From<IOError> for ReadError {
    fn from(error: IOError) -> Self {
        Self::IOError(error)
    }
}

impl Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ReadError::*;
        match self {
            IOError(error) => error.fmt(f),
            UnexpectedEOF => f.write_str("unexpected end of file"),
        }
    }
}

#[derive(ByteStruct, Debug)]
#[byte_struct_le]
pub struct FileHeader {
    pub file_version: u16,
    pub char_width: u8,
    pub char_height: u8,
    pub font_width: u8,
    pub font_height: u8,
    pub x_offset: u16,
    pub y_offset: u16,
    pub font_variant: u8
}

#[derive(ByteStruct, Debug)]
#[byte_struct_le]
struct FrameHeader {
    frame_index: u32,
    data_len: u32
}

#[derive(Debug)]
pub struct Frame {
    pub index: u32,
    pub data: Vec<u16>
}

pub struct Reader {
    file: File,
    header: FileHeader
}

impl Reader {

    fn check_signature(file: &mut File) -> Result<(), OpenError> {
        let mut signature = [0; SIGNATURE.len()];
        file.read_exact(&mut signature)?;
        if signature != SIGNATURE.as_bytes() {
            return Err(OpenError::InvalidSignature)
        }
        Ok(())
    }

    fn read_header(file: &mut File) -> Result<FileHeader, IOError> {
        let mut header_bytes = [0; FileHeader::BYTE_LEN];
        file.read_exact(&mut header_bytes)?;
        let header = FileHeader::read_bytes(&header_bytes);
        Ok(header)
    }

    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, OpenError> {
        let mut file = File::open(&path)?;
        Self::check_signature(&mut file)?;
        let header = Self::read_header(&mut file).unwrap();

        Ok(Self { file, header })
    }

    pub fn header(&self) -> &FileHeader {
        &self.header
    }

    fn read_frame_header(&mut self) -> Result<Option<FrameHeader>, ReadError> {
        let mut frame_header_bytes = [0; FrameHeader::BYTE_LEN];
        match self.file.read(&mut frame_header_bytes)? {
            0 => Ok(None),
            FrameHeader::BYTE_LEN => Ok(Some(FrameHeader::read_bytes(&frame_header_bytes))),
            _ => Err(ReadError::UnexpectedEOF)
        }
    }

    pub fn read_frame(&mut self) -> Result<Option<Frame>, ReadError> {
        let header = match self.read_frame_header()? {
            Some(header) => header,
            None => return Ok(None),
        };
        let mut data_bytes= vec![0; header.data_len as usize * 2];
        self.file.read_exact(&mut data_bytes)?;
        let data = data_bytes.chunks_exact(u16::BYTE_LEN)
            .map(|bytes| u16::from_le_bytes(bytes.try_into().unwrap())).collect();
        Ok(Some(Frame { index: header.frame_index, data }))
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