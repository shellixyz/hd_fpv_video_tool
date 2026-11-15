use std::{
	io::Error as IOError,
	path::{Path, PathBuf},
};

use clap::Args;
use derive_more::From;
use thiserror::Error;

use crate::osd::{FontDir, FontIdent, FontSource};

const DEFAULT_HOME_RELATIVE_FONT_DIR: &str = ".local/share/hd_fpv_video_tool/fonts";
const FONT_DIR_ENV_VAR_NAME: &str = "DJI_OSD_FONTS_DIR";

#[derive(Args, Clone)]
#[allow(clippy::struct_field_names, reason = "the field names are the option names")]
pub struct FontOptions {
	/// path to the directory containing font sets
	#[clap(short, long, value_parser, value_name = "dirpath")]
	font_dir: Option<PathBuf>,

	/// force using this font identifier when loading fonts, default is automatic
	#[clap(short = 'i', long, value_parser, value_name = "ident")]
	font_ident: Option<String>,

	/// load this PNG font file instead of looking up fonts in the font directory
	#[clap(long, value_parser, value_name = "filepath", conflicts_with_all = ["font_dir", "font_ident"])]
	font_file: Option<PathBuf>,
}

impl From<OSDFontOptions> for FontOptions {
	fn from(osd_font_options: OSDFontOptions) -> Self {
		Self {
			font_dir: osd_font_options.osd_font_dir,
			font_ident: osd_font_options.osd_font_ident,
			font_file: osd_font_options.osd_font_file,
		}
	}
}

impl From<&OSDFontOptions> for FontOptions {
	fn from(osd_font_options: &OSDFontOptions) -> Self {
		Self {
			font_dir: osd_font_options.osd_font_dir.clone(),
			font_ident: osd_font_options.osd_font_ident.clone(),
			font_file: osd_font_options.osd_font_file.clone(),
		}
	}
}

#[derive(Args)]
#[allow(clippy::struct_field_names, reason = "the field names are the option names")]
pub struct OSDFontOptions {
	/// path to the directory containing font sets
	#[clap(short = 'd', long, value_parser, value_name = "dirpath")]
	osd_font_dir: Option<PathBuf>,

	/// force using this font identifier when loading fonts, default is automatic
	#[clap(short = 'i', long, value_parser, value_name = "ident")]
	osd_font_ident: Option<String>,

	/// load this PNG font file instead of looking up fonts in the font directory
	#[clap(
		long,
		value_parser,
		value_name = "filepath",
		conflicts_with_all = ["osd_font_dir", "osd_font_ident"]
	)]
	osd_font_file: Option<PathBuf>,
}

#[derive(Debug, Error, From)]
pub enum OSDFontDirError {
	#[error("font dir: unable to locate home directory")]
	UnableToLocateHomeDir,
	#[error("font dir: {font_dir}: {error}")]
	CanonicalizeError { font_dir: PathBuf, error: IOError },
}

/// Resolves the OSD font directory path, using the provided path, environment variable, or default
/// path.
///
/// # Errors
/// - Returns `OSDFontDirError::UnableToLocateHomeDir` if the home directory cannot be located.
/// - Returns `OSDFontDirError::CanonicalizeError` if the font directory path cannot be
///   canonicalized.
fn font_dir_base(font_dir: Option<&Path>) -> Result<PathBuf, OSDFontDirError> {
	let font_dir = match font_dir {
		Some(font_dir) => font_dir.to_path_buf(),
		None => {
			if let Ok(font_dir) = std::env::var(FONT_DIR_ENV_VAR_NAME) {
				PathBuf::from(font_dir)
			} else {
				let home_dir = home::home_dir().ok_or(OSDFontDirError::UnableToLocateHomeDir)?;
				[home_dir, PathBuf::from(DEFAULT_HOME_RELATIVE_FONT_DIR)]
					.iter()
					.collect()
			}
		},
	};
	let font_dir = font_dir
		.canonicalize()
		.map_err(|error| OSDFontDirError::CanonicalizeError { font_dir, error })?;
	Ok(font_dir)
}

/// The font source the options pick: the explicit font file, or the font dir with the forced
/// ident, if any. An empty ident forces the generic font.
///
/// # Errors
/// See `font_dir_base` for error details; the font dir is not resolved for an explicit file.
fn font_source<'a>(
	font_dir: Option<&Path>,
	font_ident: Option<&'a str>,
	font_file: Option<&'a Path>,
) -> Result<FontSource<'a>, OSDFontDirError> {
	if let Some(font_file) = font_file {
		return Ok(FontSource::File(font_file));
	}
	let ident = match font_ident {
		None => FontIdent::FromVariant,
		Some("") => FontIdent::Generic,
		Some(ident) => FontIdent::Forced(ident),
	};
	Ok(FontSource::Dir {
		dir: FontDir::new(font_dir_base(font_dir)?),
		ident,
	})
}

impl FontOptions {
	/// The font source these options pick.
	///
	/// # Errors
	/// See `font_source` for error details.
	pub fn font_source(&self) -> Result<FontSource<'_>, OSDFontDirError> {
		font_source(
			self.font_dir.as_deref(),
			self.font_ident.as_deref(),
			self.font_file.as_deref(),
		)
	}
}

impl OSDFontOptions {
	/// The font source these options pick.
	///
	/// # Errors
	/// See `font_source` for error details.
	pub fn osd_font_source(&self) -> Result<FontSource<'_>, OSDFontDirError> {
		font_source(
			self.osd_font_dir.as_deref(),
			self.osd_font_ident.as_deref(),
			self.osd_font_file.as_deref(),
		)
	}
}

#[cfg(test)]
mod tests {
	use std::path::PathBuf;

	use clap::{Parser, error::ErrorKind};

	use super::{FontIdent, FontOptions, FontSource, OSDFontOptions, font_source};

	#[derive(Parser)]
	struct FontCli {
		#[command(flatten)]
		font_options: FontOptions,
	}

	#[derive(Parser)]
	struct OSDFontCli {
		#[command(flatten)]
		osd_font_options: OSDFontOptions,
	}

	fn parse_error_kind<P: Parser>(args: &[&str]) -> Option<ErrorKind> {
		P::try_parse_from(std::iter::once("test").chain(args.iter().copied()))
			.err()
			.map(|error| error.kind())
	}

	#[test]
	fn font_file_conflicts_with_font_ident_and_font_dir() {
		for other in [["--font-ident", "quic"], ["--font-dir", "fonts"]] {
			let args = [&["--font-file", "user_bf_24.png"][..], &other[..]].concat();
			assert_eq!(
				parse_error_kind::<FontCli>(&args),
				Some(ErrorKind::ArgumentConflict),
				"{args:?}"
			);
		}
	}

	#[test]
	fn osd_font_file_conflicts_with_osd_font_ident_and_osd_font_dir() {
		for other in [["--osd-font-ident", "quic"], ["--osd-font-dir", "fonts"]] {
			let args = [&["--osd-font-file", "user_bf_24.png"][..], &other[..]].concat();
			assert_eq!(
				parse_error_kind::<OSDFontCli>(&args),
				Some(ErrorKind::ArgumentConflict),
				"{args:?}"
			);
		}
	}

	#[test]
	fn font_file_alone_is_accepted() {
		assert_eq!(parse_error_kind::<FontCli>(&["--font-file", "user_bf_24.png"]), None);
		assert_eq!(
			parse_error_kind::<OSDFontCli>(&["--osd-font-file", "user_bf_24.png"]),
			None
		);
	}

	#[test]
	fn font_file_does_not_resolve_the_font_dir() {
		let missing_dir = PathBuf::from("/nonexistent/hd_fpv_video_tool/fonts");
		let font_file = PathBuf::from("user_bf_24.png");

		let source = font_source(Some(&missing_dir), None, Some(&font_file)).unwrap();

		assert!(matches!(source, FontSource::File(path) if path == font_file));
	}

	fn dir_ident(cli: &FontCli) -> FontIdent<'_> {
		match cli.font_options.font_source().unwrap() {
			FontSource::Dir { ident, .. } => ident,
			FontSource::File(_) => panic!("expected a font dir source"),
		}
	}

	#[test]
	fn font_ident_option_picks_the_font_ident() {
		let dir = tempfile::TempDir::new().unwrap();
		let dir_arg = dir.path().to_str().unwrap();
		let parse =
			|args: &[&str]| FontCli::try_parse_from([&["test", "--font-dir", dir_arg][..], args].concat()).unwrap();

		let (automatic, empty, quic) = (
			parse(&[]),
			parse(&["--font-ident", ""]),
			parse(&["--font-ident", "quic"]),
		);

		assert_eq!(dir_ident(&automatic), FontIdent::FromVariant);
		assert_eq!(dir_ident(&empty), FontIdent::Generic);
		assert_eq!(dir_ident(&quic), FontIdent::Forced("quic"));
	}
}
