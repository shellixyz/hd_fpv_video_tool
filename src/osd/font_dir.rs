use std::path::{Path, PathBuf};

use hd_fpv_osd_font_tool::{osd::avatar_file, prelude::*};
use itertools::Itertools;
use thiserror::Error;

use crate::osd::{font_variant::FontVariant, tile_indices::TileIndex};

#[derive(Debug, Error)]
pub enum FontLoadError {
	#[error(transparent)]
	Bin(#[from] bin_file::LoadError),
	#[error(transparent)]
	Png(#[from] avatar_file::LoadError),
	#[error("PNG font {file_path} has the {loaded} tile kind but the {requested} tile kind is needed")]
	PngTileKindMismatch {
		file_path: PathBuf,
		loaded: tile::Kind,
		requested: tile::Kind,
	},
	#[error("font file {} is not a PNG font", .0.display())]
	NotPng(PathBuf),
	#[error("no font found in {}, looked up: {}", dir.display(), looked_up.join(", "))]
	NotFound { dir: PathBuf, looked_up: Vec<String> },
}

/// One lookup step: the PNG fonts named by `png_idents`, in order, then the bin font named by
/// `bin_ident`. `None` is the generic font.
struct LookupStep<'a> {
	png_idents: Vec<Option<&'a str>>,
	bin_ident: Option<&'a str>,
}

impl LookupStep<'_> {
	fn generic() -> LookupStep<'static> {
		LookupStep {
			png_idents: vec![None],
			bin_ident: None,
		}
	}
}

/// The font ident a font dir lookup uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontIdent<'a> {
	/// The ident of the recording's font variant, with msp-osd's aliases.
	FromVariant,
	/// No ident: the generic font only.
	Generic,
	/// This ident, used as given.
	Forced(&'a str),
}

/// Where the OSD font comes from.
pub enum FontSource<'a> {
	/// Discover the font in the font dir, falling back to the generic font.
	Dir { dir: FontDir, ident: FontIdent<'a> },
	/// Load this PNG font, whatever its tile kind.
	File(&'a Path),
}

impl FontSource<'_> {
	/// Loads the font. A font dir is looked up for the given tile kind; an explicit file keeps its
	/// own tile kind, which the caller checks with `Tile::kind`.
	///
	/// # Errors
	/// Returns `FontLoadError` if no font is found or the font fails to load.
	pub fn load(
		&self,
		tile_kind: tile::Kind,
		variant: &FontVariant,
		highest_used_tile_index: TileIndex,
	) -> Result<Vec<Tile>, FontLoadError> {
		match self {
			Self::Dir { dir, ident } => match *ident {
				FontIdent::FromVariant => dir.load_variant_with_fallback(tile_kind, variant, highest_used_tile_index),
				FontIdent::Generic => dir.load_with_fallback(tile_kind, None, highest_used_tile_index),
				FontIdent::Forced(ident) => dir.load_with_fallback(tile_kind, Some(ident), highest_used_tile_index),
			},
			Self::File(file_path) => {
				let is_png = file_path
					.extension()
					.is_some_and(|extension| extension.eq_ignore_ascii_case("png"));
				if !is_png {
					return Err(FontLoadError::NotPng(file_path.to_path_buf()));
				}
				load_png(None, file_path)
			},
		}
	}
}

pub struct FontDir(PathBuf);

impl FontDir {
	pub fn new<P: AsRef<Path>>(dir_path: P) -> Self {
		Self(dir_path.as_ref().to_path_buf())
	}

	/// Loads the font for the given tile kind and font variant, falling back to the generic font
	/// when the variant has none.
	///
	/// The variant step looks up the PNG fonts named by msp-osd's idents, then the legacy aliases,
	/// then the bin font. The generic step looks up `font[_hd].png`, then the generic bin font.
	///
	/// # Errors
	/// Returns `FontLoadError` if no font is found or the font found fails to load.
	pub fn load_variant_with_fallback(
		&self,
		tile_kind: tile::Kind,
		variant: &FontVariant,
		highest_used_tile_index: TileIndex,
	) -> Result<Vec<Tile>, FontLoadError> {
		let variant_step = LookupStep {
			png_idents: variant.png_font_idents().iter().copied().map(Some).collect(),
			bin_ident: variant.font_set_ident(),
		};
		let fallback_warning = || {
			if let Some(ident) = variant.font_set_ident() {
				log::warn!("font for {variant} ({ident} ident) not found, falling back to generic font");
			}
		};
		self.load_with_generic_fallback(tile_kind, &variant_step, highest_used_tile_index, fallback_warning)
	}

	/// Loads the font with the given ident for the given tile kind, falling back to the generic
	/// font when it is missing. The ident is used as given, without aliases; `None` loads the
	/// generic font.
	///
	/// # Errors
	/// Returns `FontLoadError` if no font is found or the font found fails to load.
	pub fn load_with_fallback(
		&self,
		tile_kind: tile::Kind,
		ident: Option<&str>,
		highest_used_tile_index: TileIndex,
	) -> Result<Vec<Tile>, FontLoadError> {
		let ident_step = LookupStep {
			png_idents: ident.map(Some).into_iter().collect(),
			bin_ident: ident,
		};
		let fallback_warning = || {
			if let Some(ident) = ident {
				log::warn!("font with ident `{ident}` not found, falling back to generic font");
			}
		};
		self.load_with_generic_fallback(tile_kind, &ident_step, highest_used_tile_index, fallback_warning)
	}

	fn load_with_generic_fallback(
		&self,
		tile_kind: tile::Kind,
		first_step: &LookupStep,
		highest_used_tile_index: TileIndex,
		fallback_warning: impl FnOnce(),
	) -> Result<Vec<Tile>, FontLoadError> {
		let mut looked_up = vec![];
		if first_step.bin_ident.is_some() {
			if let Some(tiles) = self.load_step(tile_kind, first_step, highest_used_tile_index, &mut looked_up)? {
				return Ok(tiles);
			}
			fallback_warning();
		}
		if let Some(tiles) = self.load_step(
			tile_kind,
			&LookupStep::generic(),
			highest_used_tile_index,
			&mut looked_up,
		)? {
			return Ok(tiles);
		}
		Err(FontLoadError::NotFound {
			dir: self.0.clone(),
			looked_up,
		})
	}

	/// Loads the first font of the step that exists, or returns `None` when none does. The names
	/// looked up are appended to `looked_up`.
	fn load_step(
		&self,
		tile_kind: tile::Kind,
		step: &LookupStep,
		highest_used_tile_index: TileIndex,
		looked_up: &mut Vec<String>,
	) -> Result<Option<Vec<Tile>>, FontLoadError> {
		for &ident in &step.png_idents {
			let file_name = png_font_file_name(tile_kind, ident);
			let file_path = self.0.join(&file_name);
			looked_up.push(file_name);
			if file_path.exists() {
				return load_png(Some(tile_kind), &file_path).map(Some);
			}
		}
		self.load_bin(tile_kind, step.bin_ident, highest_used_tile_index, looked_up)
	}

	fn load_bin(
		&self,
		tile_kind: tile::Kind,
		ident: Option<&str>,
		highest_used_tile_index: TileIndex,
		looked_up: &mut Vec<String>,
	) -> Result<Option<Vec<Tile>>, FontLoadError> {
		let parts: &[bin_file::FontPart] = if usize::from(highest_used_tile_index) >= bin_file::TILE_COUNT {
			&[bin_file::FontPart::Base, bin_file::FontPart::Ext]
		} else {
			&[bin_file::FontPart::Base]
		};
		let file_paths = parts
			.iter()
			.map(|&part| bin_file::normalized_file_path(&self.0, tile_kind, ident, part))
			.collect::<Vec<_>>();
		let join_paths = |path_str: fn(&Path) -> String| file_paths.iter().map(|path| path_str(path)).join(" + ");
		looked_up.push(join_paths(|path| {
			path.file_name().unwrap_or_default().to_string_lossy().into_owned()
		}));
		let load_result = match parts {
			[_] => bin_file::load_base_norm(&self.0, tile_kind, ident),
			_ => bin_file::load_extended_norm(&self.0, tile_kind, ident),
		};
		match load_result {
			Ok(tiles) => {
				log::info!("loaded font {}", join_paths(|path| path.display().to_string()));
				Ok(Some(tiles))
			},
			Err(error) if error.because_file_is_missing() => Ok(None),
			Err(error) => Err(error.into()),
		}
	}
}

/// Loads a PNG font. With a tile kind, a font of another tile kind is an error.
fn load_png(tile_kind: Option<tile::Kind>, file_path: &Path) -> Result<Vec<Tile>, FontLoadError> {
	let tiles = avatar_file::load(file_path)?;
	let loaded_kind = tiles[0].kind();
	if let Some(tile_kind) = tile_kind
		&& loaded_kind != tile_kind
	{
		return Err(FontLoadError::PngTileKindMismatch {
			file_path: file_path.to_path_buf(),
			loaded: loaded_kind,
			requested: tile_kind,
		});
	}
	log::info!("loaded font {} ({} glyphs)", file_path.display(), tiles.len());
	Ok(tiles)
}

fn png_font_file_name(tile_kind: tile::Kind, ident: Option<&str>) -> String {
	let ident = ident.map(|ident| format!("_{ident}")).unwrap_or_default();
	let hd_suffix = match tile_kind {
		tile::Kind::SD => "",
		tile::Kind::HD => "_hd",
	};
	format!("font{ident}{hd_suffix}.png")
}

#[cfg(test)]
mod tests {
	use std::path::Path;

	use hd_fpv_osd_font_tool::prelude::*;
	use image::Rgba;
	use tempfile::TempDir;

	use super::{FontDir, FontIdent, FontLoadError, FontSource};
	use crate::osd::font_variant::FontVariant;

	/// Tiles whose top left pixel records the file they come from (red) and their page (green)
	fn marked_tiles(kind: tile::Kind, count: usize, marker: u8) -> Vec<Tile> {
		(0..count)
			.map(|index| {
				let mut tile = Tile::new(kind);
				#[allow(clippy::cast_possible_truncation)]
				let page = (index / 256) as u8;
				tile.put_pixel(0, 0, Rgba([marker, page, 0, 255]));
				tile
			})
			.collect()
	}

	fn write_png(dir: &Path, name: &str, kind: tile::Kind, count: usize, marker: u8) {
		marked_tiles(kind, count, marker)
			.save_to_avatar_file(dir.join(name))
			.unwrap();
	}

	fn write_bin(dir: &Path, name: &str, kind: tile::Kind, marker: u8) {
		marked_tiles(kind, 256, marker)
			.save_to_bin_file(dir.join(name))
			.unwrap();
	}

	fn marker(tiles: &[Tile]) -> u8 {
		tiles[0].get_pixel(0, 0).0[0]
	}

	fn load_variant(dir: &TempDir, kind: tile::Kind, variant: FontVariant) -> Result<Vec<Tile>, FontLoadError> {
		FontDir::new(dir.path()).load_variant_with_fallback(kind, &variant, 10)
	}

	#[test]
	fn bin_only_font_dir_loads_the_variant_bin_font() {
		let dir = TempDir::new().unwrap();
		write_bin(dir.path(), "font_bf.bin", tile::Kind::SD, 1);
		write_bin(dir.path(), "font.bin", tile::Kind::SD, 2);

		let tiles = load_variant(&dir, tile::Kind::SD, FontVariant::Betaflight).unwrap();

		assert_eq!(marker(&tiles), 1);
		assert_eq!(tiles.len(), 256);
	}

	#[test]
	fn variant_lookup_order_is_png_name_alias_bin_then_generic_png_bin() {
		let dir = TempDir::new().unwrap();
		let files = [
			("font_btfl.png", 1),
			("font_bf.png", 2),
			("font_bf.bin", 3),
			("font.png", 4),
			("font.bin", 5),
		];
		for (name, marker) in files {
			if Path::new(name).extension().is_some_and(|extension| extension == "png") {
				write_png(dir.path(), name, tile::Kind::SD, 256, marker);
			} else {
				write_bin(dir.path(), name, tile::Kind::SD, marker);
			}
		}

		for (name, expected_marker) in files {
			let tiles = load_variant(&dir, tile::Kind::SD, FontVariant::Betaflight).unwrap();
			assert_eq!(marker(&tiles), expected_marker, "expected {name} to be loaded");
			std::fs::remove_file(dir.path().join(name)).unwrap();
		}
	}

	#[test]
	fn kiss_ultra_falls_back_to_the_ultra_alias() {
		let dir = TempDir::new().unwrap();
		write_png(dir.path(), "font_ultra.png", tile::Kind::SD, 256, 1);
		write_png(dir.path(), "font.png", tile::Kind::SD, 256, 2);

		let tiles = load_variant(&dir, tile::Kind::SD, FontVariant::KISSUltra).unwrap();

		assert_eq!(marker(&tiles), 1);
	}

	#[test]
	fn hd_tile_kind_looks_up_hd_names() {
		let dir = TempDir::new().unwrap();
		write_png(dir.path(), "font_inav.png", tile::Kind::SD, 256, 1);
		write_png(dir.path(), "font_inav_hd.png", tile::Kind::HD, 256, 2);

		let tiles = load_variant(&dir, tile::Kind::HD, FontVariant::INAV).unwrap();

		assert_eq!(marker(&tiles), 2);
		assert_eq!(tiles[0].kind(), tile::Kind::HD);
	}

	#[test]
	fn forced_ident_is_used_as_given_without_aliases() {
		let dir = TempDir::new().unwrap();
		write_png(dir.path(), "font_bf.png", tile::Kind::SD, 256, 1);
		write_png(dir.path(), "font_quic.png", tile::Kind::SD, 256, 2);
		write_png(dir.path(), "font.png", tile::Kind::SD, 256, 3);
		let font_dir = FontDir::new(dir.path());

		let quic = font_dir.load_with_fallback(tile::Kind::SD, Some("quic"), 10).unwrap();
		let btfl = font_dir.load_with_fallback(tile::Kind::SD, Some("btfl"), 10).unwrap();

		assert_eq!(marker(&quic), 2);
		assert_eq!(marker(&btfl), 3);
	}

	#[test]
	fn forced_ident_prefers_png_over_bin() {
		let dir = TempDir::new().unwrap();
		write_bin(dir.path(), "font_quic.bin", tile::Kind::SD, 1);
		write_png(dir.path(), "font_quic.png", tile::Kind::SD, 256, 2);

		let tiles = FontDir::new(dir.path())
			.load_with_fallback(tile::Kind::SD, Some("quic"), 10)
			.unwrap();

		assert_eq!(marker(&tiles), 2);
	}

	#[test]
	fn generic_lookup_prefers_png_over_bin() {
		let dir = TempDir::new().unwrap();
		write_bin(dir.path(), "font.bin", tile::Kind::SD, 1);
		write_png(dir.path(), "font.png", tile::Kind::SD, 256, 2);

		let tiles = load_variant(&dir, tile::Kind::SD, FontVariant::Generic).unwrap();

		assert_eq!(marker(&tiles), 2);
	}

	#[test]
	fn loads_all_pages_of_a_four_page_png_font() {
		let dir = TempDir::new().unwrap();
		write_png(dir.path(), "font_btfl.png", tile::Kind::SD, 1024, 1);

		let tiles = FontDir::new(dir.path())
			.load_variant_with_fallback(tile::Kind::SD, &FontVariant::Betaflight, 1000)
			.unwrap();

		assert_eq!(tiles.len(), 1024);
		assert_eq!(tiles[768].get_pixel(0, 0).0, [1, 3, 0, 255]);
	}

	#[test]
	fn glyph_index_256_loads_the_extended_bin_part() {
		let dir = TempDir::new().unwrap();
		write_bin(dir.path(), "font.bin", tile::Kind::SD, 1);
		write_bin(dir.path(), "font_2.bin", tile::Kind::SD, 2);

		let tiles = FontDir::new(dir.path())
			.load_with_fallback(tile::Kind::SD, None, 256)
			.unwrap();

		assert_eq!(tiles.len(), 512);
	}

	#[test]
	fn png_font_with_the_wrong_tile_kind_is_an_error() {
		let dir = TempDir::new().unwrap();
		write_png(dir.path(), "font_hd.png", tile::Kind::SD, 256, 1);

		let error = load_variant(&dir, tile::Kind::HD, FontVariant::Generic).unwrap_err();

		assert!(
			matches!(error, FontLoadError::PngTileKindMismatch { .. }),
			"unexpected error: {error}"
		);
	}

	#[test]
	fn missing_font_error_lists_the_files_looked_up() {
		let dir = TempDir::new().unwrap();

		let error = load_variant(&dir, tile::Kind::HD, FontVariant::KISSUltra).unwrap_err();

		assert_eq!(
			error.to_string(),
			format!(
				"no font found in {}, looked up: font_ultr_hd.png, font_ultra_hd.png, font_ultra_hd.bin, font_hd.png, \
				 font_hd.bin",
				dir.path().display()
			)
		);
	}

	#[test]
	fn explicit_font_file_is_loaded_with_its_own_tile_kind() {
		let dir = TempDir::new().unwrap();
		write_png(dir.path(), "font_btfl.png", tile::Kind::SD, 256, 1);
		write_png(dir.path(), "user_bf_24.png", tile::Kind::HD, 512, 2);
		let font_file = dir.path().join("user_bf_24.png");

		let tiles = FontSource::File(&font_file)
			.load(tile::Kind::SD, &FontVariant::Betaflight, 10)
			.unwrap();

		assert_eq!(marker(&tiles), 2);
		assert_eq!(tiles.len(), 512);
		assert_eq!(tiles[0].kind(), tile::Kind::HD);
	}

	#[test]
	fn font_dir_source_loads_the_font_its_ident_picks() {
		let dir = TempDir::new().unwrap();
		write_png(dir.path(), "font_btfl.png", tile::Kind::SD, 256, 1);
		write_png(dir.path(), "font_quic.png", tile::Kind::SD, 256, 2);
		write_png(dir.path(), "font.png", tile::Kind::SD, 256, 3);
		let load = |ident| {
			FontSource::Dir {
				dir: FontDir::new(dir.path()),
				ident,
			}
			.load(tile::Kind::SD, &FontVariant::Betaflight, 10)
			.unwrap()
		};

		assert_eq!(marker(&load(FontIdent::FromVariant)), 1);
		assert_eq!(marker(&load(FontIdent::Forced("quic"))), 2);
		assert_eq!(marker(&load(FontIdent::Generic)), 3);
	}

	#[test]
	fn explicit_font_file_must_be_a_png() {
		let dir = TempDir::new().unwrap();
		write_png(dir.path(), "user_bf_24.png", tile::Kind::HD, 256, 1);
		let jpg_path = dir.path().join("user_bf_24.jpg");
		image::open(dir.path().join("user_bf_24.png"))
			.unwrap()
			.to_rgb8()
			.save(&jpg_path)
			.unwrap();

		let error = FontSource::File(&jpg_path)
			.load(tile::Kind::HD, &FontVariant::Generic, 10)
			.unwrap_err();

		assert!(matches!(error, FontLoadError::NotPng(_)), "unexpected error: {error}");
	}
}
