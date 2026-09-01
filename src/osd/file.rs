use std::{
	io::Error as IOError,
	path::{Path, PathBuf},
};

use ambassador::{Delegate, delegatable_trait};
use derive_more::From;
use thiserror::Error;

pub mod frame;
pub mod sorted_frames;

pub use frame::Frame;

pub use self::sorted_frames::SortedUniqFrames;
use super::{FontVariant, tile_indices::TileIndex};

#[derive(Debug, Error, From)]
pub enum ReadError {
	#[error(transparent)]
	FileError(IOError),
	#[error("Unexpected end of file: {file_path}")]
	UnexpectedEOF { file_path: PathBuf },
}

impl ReadError {
	pub fn unexpected_eof<P: AsRef<Path>>(file_path: P) -> Self {
		Self::UnexpectedEOF {
			file_path: file_path.as_ref().to_path_buf(),
		}
	}
}

#[delegatable_trait]
pub trait GenericReader {
	/// Reads the next frame from the OSD file. Returns `Ok(None)` if there are no more frames to
	/// read.
	///
	/// # Errors
	/// Returns a `ReadError` if there is an error reading the frame.
	fn read_frame(&mut self) -> Result<Option<Frame>, ReadError>;

	/// Returns all frames in the OSD file as a `SortedUniqFrames` collection.
	///
	/// # Errors
	/// Returns a `ReadError` if there is an error reading the frames.
	fn frames(&mut self) -> Result<SortedUniqFrames, ReadError>;

	/// Returns the index of the last frame in the OSD file.
	///
	/// # Errors
	/// Returns a `ReadError` if there is an error reading the last frame index.
	fn last_frame_frame_index(&mut self) -> Result<u32, ReadError>;

	/// Returns the highest used tile index in the OSD file.
	///
	/// # Errors
	/// Returns a `ReadError` if there is an error reading the highest used tile index.
	fn max_used_tile_index(&mut self) -> Result<TileIndex, ReadError>;

	/// Returns the font variant used in the OSD file.
	///
	/// # Errors
	/// Returns a `ReadError` if there is an error reading the font variant.
	fn font_variant(&self) -> FontVariant;
}

pub fn find_associated_to_video_file<P: AsRef<Path>>(video_file_path: P) -> Option<PathBuf> {
	let video_file_path = video_file_path.as_ref();
	log::info!(
		"looking for OSD file associated to video file: {}",
		video_file_path.to_string_lossy()
	);

	let osd_file_path = video_file_path.with_extension("osd");
	if osd_file_path.is_file() {
		log::info!("found: {}", osd_file_path.to_string_lossy());
		return Some(osd_file_path);
	}
	log::info!("not found: {}", osd_file_path.to_string_lossy());

	let file_stem = video_file_path.file_stem()?.to_string_lossy();

	if file_stem.starts_with("DJI") {
		super::dji::file::find_associated_to_video_file(video_file_path)
	} else if file_stem.starts_with("Avatar") {
		super::wsa::file::find_associated_to_video_file(video_file_path)
	} else {
		None
	}
}

#[derive(Delegate)]
#[delegate(GenericReader)]
pub enum Reader {
	DJI(crate::osd::dji::file::Reader),
	WSA(crate::osd::wsa::file::Reader),
}

#[derive(Debug, Error)]
#[error("unrecognized OSD file: {0}")]
pub struct UnrecognizedOSDFile(PathBuf);

/// Opens an OSD file and returns a `Reader` for it.
///
/// # Errors
/// Returns an `UnrecognizedOSDFile` error if the file format is not recognized.
pub fn open(path: impl AsRef<Path>) -> Result<Reader, UnrecognizedOSDFile> {
	let path = path.as_ref();
	if let Some(file_stem) = path.file_stem() {
		let file_stem = file_stem.to_string_lossy();
		if file_stem.starts_with("DJIG") {
			if let Ok(reader) = super::dji::file::Reader::open(path) {
				return Ok(Reader::DJI(reader));
			}
		} else if file_stem.starts_with("AvatarG") {
			if let Ok(reader) = super::wsa::file::Reader::open(path) {
				return Ok(Reader::WSA(reader));
			}
		}
	}

	if let Ok(reader) = super::dji::file::Reader::open(path) {
		return Ok(Reader::DJI(reader));
	}

	if let Ok(reader) = super::wsa::file::Reader::open(path) {
		return Ok(Reader::WSA(reader));
	}

	Err(UnrecognizedOSDFile(path.to_owned()))
}
