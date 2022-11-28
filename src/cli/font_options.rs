
use std::path::PathBuf;

use clap::Args;

const DEFAULT_FONT_DIR: &str = "fonts";
const FONT_DIR_ENV_VAR_NAME: &str = "DJI_OSD_FONTS_DIR";

#[derive(Args)]
pub struct FontOptions {
    /// path to the directory containing font sets
    #[clap(short, long, value_parser, value_name = "dirpath")]
    font_dir: Option<PathBuf>,

    /// force using this font identifier when loading fonts, default is automatic
    #[clap(short = 'i', long, value_parser, value_name = "ident")]
    font_ident: Option<String>,
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

impl FontOptions {

    pub fn font_dir(&self) -> PathBuf {
        self.font_dir.clone().unwrap_or_else(||
            PathBuf::from(
                std::env::var(FONT_DIR_ENV_VAR_NAME)
                    .unwrap_or_else(|_| DEFAULT_FONT_DIR.to_owned())
            )
        )
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

    pub fn osd_font_dir(&self) -> PathBuf {
        self.osd_font_dir.clone().unwrap_or_else(||
            PathBuf::from(
                std::env::var(FONT_DIR_ENV_VAR_NAME)
                    .unwrap_or_else(|_| DEFAULT_FONT_DIR.to_owned())
            )
        )
    }

    pub fn osd_font_ident(&self) -> Option<Option<&str>> {
        match self.osd_font_ident.as_deref() {
            Some("") => Some(None),
            Some(font_ident_str) => Some(Some(font_ident_str)),
            None => None,
        }
    }

}
