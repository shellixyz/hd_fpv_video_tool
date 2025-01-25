use std::io::Error as IOError;
use std::path::PathBuf;

use clap::Args;
use derive_more::From;
use thiserror::Error;

const DEFAULT_HOME_RELATIVE_FONT_DIR: &str = ".local/share/hd_fpv_video_tool/fonts";
const FONT_DIR_ENV_VAR_NAME: &str = "DJI_OSD_FONTS_DIR";

#[derive(Args, Clone)]
pub struct FontOptions {
	/// path to the directory containing font sets
	#[clap(short, long, value_parser, value_name = "dirpath")]
	font_dir: Option<PathBuf>,

	/// force using this font identifier when loading fonts, default is automatic
	#[clap(short = 'i', long, value_parser, value_name = "ident")]
	font_ident: Option<String>,
}

impl From<OSDFontOptions> for FontOptions {
	fn from(osd_font_options: OSDFontOptions) -> Self {
		Self {
			font_dir: osd_font_options.osd_font_dir,
			font_ident: osd_font_options.osd_font_ident,
		}
	}
}

impl From<&OSDFontOptions> for FontOptions {
	fn from(osd_font_options: &OSDFontOptions) -> Self {
		Self {
			font_dir: osd_font_options.osd_font_dir.clone(),
			font_ident: osd_font_options.osd_font_ident.clone(),
		}
	}
}

#[derive(Args)]
pub struct OSDFontOptions {
	/// path to the directory containing font sets
	#[clap(short = 'd', long, value_parser, value_name = "dirpath")]
	osd_font_dir: Option<PathBuf>,

	/// force using this font identifier when loading fonts, default is automatic
	#[clap(short = 'i', long, value_parser, value_name = "ident")]
	osd_font_ident: Option<String>,
}

#[derive(Debug, Error, From)]
pub enum OSDFontDirError {
	#[error("font dir: unable to locate home directory")]
	UnableToLocateHomeDir,
	#[error("font dir: {font_dir}: {error}")]
	CanonicalizeError { font_dir: PathBuf, error: IOError },
}

fn font_dir_base(font_dir: &Option<PathBuf>) -> Result<PathBuf, OSDFontDirError> {
	let font_dir = match font_dir {
		Some(font_dir) => font_dir.clone(),
		None => match std::env::var(FONT_DIR_ENV_VAR_NAME) {
			Ok(font_dir) => PathBuf::from(font_dir),
			Err(_) => {
				let home_dir = home::home_dir().ok_or(OSDFontDirError::UnableToLocateHomeDir)?;
				[home_dir, PathBuf::from(DEFAULT_HOME_RELATIVE_FONT_DIR)]
					.iter()
					.collect()
			},
		},
	};
	let font_dir = font_dir
		.canonicalize()
		.map_err(|error| OSDFontDirError::CanonicalizeError { font_dir, error })?;
	Ok(font_dir)
}

impl FontOptions {
	pub fn font_dir(&self) -> Result<PathBuf, OSDFontDirError> {
		font_dir_base(&self.font_dir)
	}

	pub fn font_ident(&self) -> Option<Option<&str>> {
		match self.font_ident.as_deref() {
			Some("") => Some(None),
			Some(font_ident_str) => Some(Some(font_ident_str)),
			None => None,
		}
	}
}

impl OSDFontOptions {
	pub fn osd_font_dir(&self) -> Result<PathBuf, OSDFontDirError> {
		font_dir_base(&self.osd_font_dir)
	}

	pub fn osd_font_ident(&self) -> Option<Option<&str>> {
		match self.osd_font_ident.as_deref() {
			Some("") => Some(None),
			Some(font_ident_str) => Some(Some(font_ident_str)),
			None => None,
		}
	}
}
