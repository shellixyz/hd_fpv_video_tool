use std::{
	fmt::Display,
	io::{Error as IOError, Read, Seek, SeekFrom},
	path::{Path, PathBuf},
	sync::LazyLock,
};

use byte_struct::{ByteStruct, ByteStructLen, ByteStructUnspecifiedByteOrder};
use derive_more::From;
use fs_err::File;
use getset::{CopyGetters, Getters};
use hd_fpv_osd_font_tool::prelude::*;
use itertools::Itertools;
use regex::Regex;
use thiserror::Error;

use crate::{
	osd::{
		Dimensions, FontVariant, Kind, TileIndices,
		file::{Frame, GenericReader, ReadError, sorted_frames::SortedUniqFrames},
		kind::InvalidDimensionsError,
		tile_indices::TileIndex,
	},
	video::FrameIndex as VideoFrameIndex,
};

const SIGNATURE: &str = "MSPOSD\x00";

/// The font variant field that ends the file header, which depends on the format version.
#[derive(Debug, Clone, Copy)]
enum FontVariantField {
	/// Format v1: a one-byte numeric ID.
	NumericId,
	/// Format v2 (msp-osd 0.12 and later): an MSP FC variant ID, NUL-terminated unless it fills
	/// its 5 bytes. msp-osd records an empty ID for the generic font.
	MspFcVariantId,
}

impl FontVariantField {
	const fn of_format_version(format_version: u16) -> Option<Self> {
		match format_version {
			1 => Some(Self::NumericId),
			2 => Some(Self::MspFcVariantId),
			_ => None,
		}
	}

	const fn len(self) -> usize {
		match self {
			Self::NumericId => 1,
			Self::MspFcVariantId => 5,
		}
	}

	/// Returns the recorded ID as text and the font variant it selects.
	fn parse(self, bytes: &[u8]) -> (String, FontVariant) {
		use FontVariant as FV;
		match self {
			Self::NumericId => {
				let id = bytes[0];
				let variant = match id {
					0 => FV::Generic,
					1 => FV::Betaflight,
					2 => FV::INAV,
					3 => FV::Ardupilot,
					4 => FV::KISSUltra,
					5 => FV::Quicksilver,
					_ => FV::Unknown,
				};
				(id.to_string(), variant)
			},
			Self::MspFcVariantId => {
				let id = bytes.split(|byte| *byte == 0).next().unwrap_or_default();
				let id = String::from_utf8_lossy(id).into_owned();
				let variant = if id.is_empty() {
					FV::Generic
				} else {
					FV::from_msp_fc_variant_id(&id)
				};
				(id, variant)
			},
		}
	}
}

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
		Self::InvalidSignature {
			file_path: file_path.as_ref().to_path_buf(),
		}
	}

	fn invalid_osd_dimensions<P: AsRef<Path>>(file_path: P, dimensions: Dimensions) -> Self {
		Self::InvalidOSDDimensions {
			file_path: file_path.as_ref().to_path_buf(),
			dimensions,
		}
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
}

#[derive(Debug, Getters)]
#[getset(get = "pub")]
pub struct Offset {
	x: u16,
	y: u16,
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
	/// The recorded font variant ID: a number in format v1, a string in format v2.
	font_variant_id: String,
	#[getset(skip)]
	font_variant: FontVariant,
}

impl FileHeader {
	#[must_use]
	pub fn font_variant(&self) -> FontVariant {
		self.font_variant
	}

	fn new(fhr: &FileHeaderRaw, font_variant_field: FontVariantField, font_variant_bytes: &[u8]) -> Self {
		let (font_variant_id, font_variant) = font_variant_field.parse(font_variant_bytes);
		Self {
			format_version: fhr.format_version,
			osd_dimensions: Dimensions::new(u32::from(fhr.width_tiles), u32::from(fhr.height_tiles)),
			tile_dimensions: TileDimensions {
				width: u32::from(fhr.tile_width),
				height: u32::from(fhr.tile_height),
			},
			offset: Offset {
				x: fhr.x_offset,
				y: fhr.y_offset,
			},
			font_variant_id,
			font_variant,
		}
	}
}

#[derive(ByteStruct, Debug, CopyGetters)]
#[getset(get_copy = "pub")]
#[byte_struct_le]
pub struct FrameHeader {
	frame_index: VideoFrameIndex,
	data_len: u32,
}

#[derive(Getters, CopyGetters)]
pub struct Reader {
	file: File,
	#[getset(get = "pub")]
	header: FileHeader,
	#[getset(get_copy = "pub")]
	osd_kind: Kind,
	first_frame_file_pos: u64,
}

impl Reader {
	fn check_signature<P: AsRef<Path>>(file_path: P, file: &mut File) -> Result<(), OpenError> {
		let mut signature = [0; SIGNATURE.len()];
		file.read_exact(&mut signature)?;
		if signature != SIGNATURE.as_bytes() {
			return Err(OpenError::invalid_signature(&file_path));
		}
		Ok(())
	}

	fn read_header(file: &mut File) -> Result<FileHeader, OpenError> {
		let mut header_bytes = [0; FileHeaderRaw::BYTE_LEN];
		file.read_exact(&mut header_bytes)?;
		let header = FileHeaderRaw::read_bytes(&header_bytes);
		let Some(font_variant_field) = FontVariantField::of_format_version(header.format_version) else {
			return Err(OpenError::UnsupportedFileFormatVersion(header.format_version));
		};
		let mut font_variant_bytes = vec![0; font_variant_field.len()];
		file.read_exact(&mut font_variant_bytes)?;
		Ok(FileHeader::new(&header, font_variant_field, &font_variant_bytes))
	}

	/// Opens a DJI OSD file for reading.
	///
	/// # Errors
	/// - Returns `OpenError::FileError` if there is an error opening or reading the file.
	/// - Returns `OpenError::InvalidSignature` if the file signature is invalid.
	/// - Returns `OpenError::UnsupportedFileFormatVersion` if the format version is not 1 or 2.
	/// - Returns `OpenError::InvalidOSDDimensions` if the OSD dimensions are invalid.
	pub fn open<P: AsRef<Path>>(file_path: P) -> Result<Self, OpenError> {
		let mut file = File::open(&file_path)?;
		Self::check_signature(&file_path, &mut file)?;
		let header = Self::read_header(&mut file)?;
		let first_frame_file_pos = file.stream_position()?;
		let osd_kind = Kind::try_from(header.osd_dimensions()).map_err(|error| {
			let InvalidDimensionsError(dimensions) = error;
			OpenError::invalid_osd_dimensions(&file_path, dimensions)
		})?;
		log::info!("detected OSD file with {osd_kind} tile layout");
		Ok(Self {
			file,
			header,
			osd_kind,
			first_frame_file_pos,
		})
	}

	fn read_frame_header(&mut self) -> Result<Option<FrameHeader>, ReadError> {
		let mut frame_header_bytes = [0; FrameHeader::BYTE_LEN];
		match self.file.read(&mut frame_header_bytes)? {
			0 => Ok(None),
			FrameHeader::BYTE_LEN => Ok(Some(FrameHeader::read_bytes(&frame_header_bytes))),
			_ => {
				log::warn!("unexpected end of file while reading frame header");
				Ok(None)
			},
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
	//     let frames =
	// frames.into_iter().sorted_unstable_by_key(Frame::index).unique_by(Frame::index).collect();
	//     Ok(SortedUniqFrames::new(osd_kind, font_variant, frames))
	// }

	/// Rewinds the reader to the beginning of the first frame.
	///
	/// # Errors
	/// - Returns `IOError` if there is an error seeking in the file.
	pub fn rewind(&mut self) -> Result<(), IOError> {
		self.file.seek(SeekFrom::Start(self.first_frame_file_pos))?;
		Ok(())
	}

	fn keep_position_do<F, X, E>(&mut self, f: F) -> Result<X, E>
	where
		F: FnOnce(&mut Self) -> Result<X, E>,
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

	pub fn iter(&mut self) -> Iter<'_> {
		self.into_iter()
	}

	pub fn iter_mut(&mut self) -> Iter<'_> {
		self.into_iter()
	}
}

impl GenericReader for Reader {
	fn read_frame(&mut self) -> Result<Option<Frame>, ReadError> {
		let Some(header) = self.read_frame_header()? else {
			return Ok(None);
		};
		let mut data_bytes = vec![0; header.data_len() as usize * 2];
		match self.file.read_exact(&mut data_bytes) {
			Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
				log::warn!("unexpected end of file while reading frame data");
				return Ok(None);
			},
			Err(e) => return Err(ReadError::FileError(e)),
			Ok(()) => {},
		}
		let tile_indices = TileIndices::new(
			data_bytes
				.as_chunks::<{ u16::BYTE_LEN }>()
				.0
				.iter()
				.map(|bytes| u16::from_le_bytes(*bytes))
				.collect(),
		);
		Ok(Some(Frame::new(header.frame_index(), tile_indices)))
	}

	fn frames(&mut self) -> Result<SortedUniqFrames, ReadError> {
		self.rewind()?;
		let osd_kind = self.osd_kind;
		let font_variant = self.header.font_variant();
		let mut frames = vec![];
		let osd_dimensions = self.header.osd_dimensions;
		for frame_read_result in self {
			frames.push(frame_read_result?);
		}
		let frames = frames
			.into_iter()
			.sorted_unstable_by_key(Frame::index)
			.unique_by(Frame::index)
			.collect::<Vec<Frame>>();
		'outer: for frame in &frames {
			for (coordinates, tile_index) in frame.enumerate_tile_indices() {
				if tile_index > 0
					&& (u32::from(coordinates.x) >= osd_dimensions.width
						|| u32::from(coordinates.y) >= osd_dimensions.height)
				{
					log::warn!(
						"the OSD dimensions in the OSD file header do not seem to match the actual data in the file, \
						 the OSD might not be rendered fully"
					);
					break 'outer;
				}
			}
		}
		Ok(SortedUniqFrames::new(osd_kind, font_variant, frames))
	}

	fn last_frame_frame_index(&mut self) -> Result<u32, ReadError> {
		self.keep_position_do(|reader| Ok(reader.frames()?.last().unwrap().index()))
	}

	fn max_used_tile_index(&mut self) -> Result<TileIndex, ReadError> {
		self.keep_position_do(|reader| {
			Ok(*reader
				.frames()?
				.iter()
				.flat_map(|frame| frame.tile_indices().as_slice())
				.max()
				.unwrap())
		})
	}

	fn font_variant(&self) -> FontVariant {
		self.header.font_variant()
	}
}

pub struct IntoIter {
	reader: Reader,
}

impl Iterator for IntoIter {
	type Item = Result<Frame, ReadError>;

	fn next(&mut self) -> Option<Self::Item> {
		self.reader.read_frame().transpose()
	}
}

impl IntoIterator for Reader {
	type IntoIter = IntoIter;
	type Item = Result<Frame, ReadError>;

	fn into_iter(self) -> Self::IntoIter {
		Self::IntoIter { reader: self }
	}
}

pub struct Iter<'a> {
	reader: &'a mut Reader,
}

impl Iterator for Iter<'_> {
	type Item = Result<Frame, ReadError>;

	fn next(&mut self) -> Option<Self::Item> {
		self.reader.read_frame().transpose()
	}
}

impl<'a> IntoIterator for &'a mut Reader {
	type IntoIter = Iter<'a>;
	type Item = Result<Frame, ReadError>;

	fn into_iter(self) -> Self::IntoIter {
		Self::IntoIter { reader: self }
	}
}

/// Attempts to find an associated DJI OSD file for the given video file path.
pub fn find_associated_to_video_file<P: AsRef<Path>>(video_file_path: P) -> Option<PathBuf> {
	static DJI_VIDEO_FILE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\A(?:DJI(?:G|U)(\d{4}))").unwrap());

	let video_file_path = video_file_path.as_ref();
	let file_stem = video_file_path.file_stem()?.to_string_lossy();

	if let Some(captures) = DJI_VIDEO_FILE_RE.captures(&file_stem) {
		let Some(dji_file_number) = captures.get(1) else {
			unreachable!();
		};
		let osd_file_path = video_file_path
			.with_file_name(format!("DJIG{}", dji_file_number.as_str()))
			.with_extension("osd");
		if osd_file_path.is_file() {
			log::info!("found: {}", osd_file_path.to_string_lossy());
			return Some(osd_file_path);
		}
		log::info!("not found: {}", osd_file_path.to_string_lossy());
	}

	None
}

#[cfg(test)]
mod tests {
	use std::io::Write;

	use tempfile::NamedTempFile;

	use super::{OpenError, Reader, SIGNATURE};
	use crate::osd::{FontVariant, file::GenericReader};

	const FRAME_INDEX: u32 = 7;
	const TILES: [u16; 3] = [0x41, 0, 0x142];

	/// Writes an OSD file with the msp-osd `rec.h` layout: signature, version, config with the
	/// given font variant bytes, then one frame.
	fn osd_file(version: u16, font_variant: &[u8]) -> NamedTempFile {
		let mut bytes = SIGNATURE.as_bytes().to_vec();
		bytes.extend(version.to_le_bytes());
		bytes.extend([50, 18, 36, 54]);
		bytes.extend(10_u16.to_le_bytes());
		bytes.extend(20_u16.to_le_bytes());
		bytes.extend(font_variant);
		bytes.extend(FRAME_INDEX.to_le_bytes());
		bytes.extend(u32::try_from(TILES.len()).unwrap().to_le_bytes());
		for tile in TILES {
			bytes.extend(tile.to_le_bytes());
		}
		let mut file = NamedTempFile::new().unwrap();
		file.write_all(&bytes).unwrap();
		file
	}

	fn open(version: u16, font_variant: &[u8]) -> Reader {
		Reader::open(osd_file(version, font_variant).path()).unwrap()
	}

	fn assert_reads_the_frame(reader: &mut Reader) {
		let frames = reader.frames().unwrap();
		assert_eq!(frames.len(), 1);
		assert_eq!(frames[0].index(), FRAME_INDEX);
		assert_eq!(&frames[0].tile_indices().as_slice()[..TILES.len()], TILES);
	}

	#[test]
	fn v1_file_reads_its_variant_id_and_frames() {
		let mut reader = open(1, &[4]);
		assert_eq!(reader.header().font_variant(), FontVariant::KISSUltra);
		assert_eq!(reader.header().font_variant_id(), "4");
		assert_eq!(*reader.header().offset().x(), 10);
		assert_eq!(*reader.header().offset().y(), 20);
		assert_reads_the_frame(&mut reader);
	}

	#[test]
	fn v1_variant_id_5_is_quicksilver() {
		assert_eq!(open(1, &[5]).header().font_variant(), FontVariant::Quicksilver);
	}

	#[test]
	fn v2_file_reads_its_variant_string_and_frames_after_the_longer_header() {
		let mut reader = open(2, b"QUIC\0");
		assert_eq!(reader.header().format_version(), &2);
		assert_eq!(reader.header().font_variant(), FontVariant::Quicksilver);
		assert_eq!(reader.header().font_variant_id(), "QUIC");
		assert_reads_the_frame(&mut reader);
	}

	#[test]
	fn v2_variant_string_may_fill_the_field_without_nul() {
		let reader = open(2, b"ABCDE");
		assert_eq!(reader.header().font_variant(), FontVariant::Unknown);
		assert_eq!(reader.header().font_variant_id(), "ABCDE");
	}

	#[test]
	fn v2_empty_variant_string_is_generic() {
		// msp-osd's playback turns v1 ID 0 (generic) into an empty string.
		assert_eq!(open(2, &[0; 5]).header().font_variant(), FontVariant::Generic);
	}

	#[test]
	fn version_3_is_unsupported() {
		let file = osd_file(3, &[0; 5]);
		assert!(matches!(
			Reader::open(file.path()),
			Err(OpenError::UnsupportedFileFormatVersion(3))
		));
	}
}
