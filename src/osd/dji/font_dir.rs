
use std::path::{PathBuf, Path};

use hd_fpv_osd_font_tool::prelude::*;

use super::file::{TileIndex, FontVariant};


pub struct FontDir(PathBuf);

impl FontDir {

    pub fn new<P: AsRef<Path>>(dir_path: P) -> Self {
        Self(dir_path.as_ref().to_path_buf())
    }

    pub fn load(&self, tile_kind: tile::Kind, ident: &Option<&str>, max_used_tile_index: TileIndex) -> Result<Vec<Tile>, bin_file::LoadError> {
        match max_used_tile_index {
            max_index if max_index <= bin_file::TILE_COUNT as u16 => bin_file::load_base_norm(&self.0, tile_kind, ident),
            _ => bin_file::load_extended_norm(&self.0, tile_kind, ident)
        }
    }

    pub fn load_variant_with_fallback(&self, tile_kind: tile::Kind, variant: &FontVariant, max_used_tile_index: TileIndex) -> Result<Vec<Tile>, bin_file::LoadError> {
        let ident = variant.font_set_ident();
        let ident_load_result = self.load(tile_kind, &ident, max_used_tile_index);
        let tiles = match (ident, ident_load_result) {
            (None, Ok(tiles)) | (Some(_), Ok(tiles)) => tiles,
            (None, error @ Err(_)) => return error,
            (Some(ident), Err(error)) => {
                if error.because_file_is_missing() {
                    log::warn!("font for {variant} ({ident} ident) not found, falling back to generic font");
                    self.load(tile_kind, &None, max_used_tile_index)?
                } else {
                    return Err(error);
                }
            },
        };
        Ok(tiles)
    }

    pub fn load_with_fallback(&self, tile_kind: tile::Kind, ident: &Option<&str>, max_used_tile_index: TileIndex) -> Result<Vec<Tile>, bin_file::LoadError> {
        let ident_load_result = self.load(tile_kind, ident, max_used_tile_index);
        let tiles = match (ident, ident_load_result) {
            (None, Ok(tiles)) | (Some(_), Ok(tiles)) => tiles,
            (None, error @ Err(_)) => return error,
            (Some(ident), Err(error)) => {
                if error.because_file_is_missing() {
                    log::warn!("font with ident `{ident}` not found, falling back to generic font");
                    self.load(tile_kind, &None, max_used_tile_index)?
                } else {
                    return Err(error);
                }
            },
        };
        Ok(tiles)
    }

}