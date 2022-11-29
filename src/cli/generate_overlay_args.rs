use std::{path::PathBuf, ffi::OsStr};

use clap::Args;
use getset::{Getters, CopyGetters};
use anyhow::anyhow;

use crate::prelude::ScalingArgs;

use super::{font_options::FontOptions, start_end_args::StartEndArgs};


#[derive(Args, Getters, CopyGetters)]
#[getset(get = "pub")]
pub struct GenerateOverlayArgs {

    #[clap(flatten)]
    start_end: StartEndArgs,

    #[clap(flatten)]
    scaling_args: ScalingArgs,

    #[clap(flatten)]
    font_options: FontOptions,

    /// Shift the output by that number of frames. Use this option to sync the OSD to a particular video.
    #[clap(short = 'o', long, value_parser, value_name = "frames", allow_negative_numbers(true), default_value_t = 0)]
    #[getset(skip)]
    #[getset(get_copy = "pub")]
    frame_shift: i32,

    /// path to FPV.WTF .osd file
    osd_file: PathBuf,

}

impl GenerateOverlayArgs {
    pub fn check_valid(&self) -> anyhow::Result<()> {
        self.start_end().check_valid()?;
        if self.osd_file.extension().map(ToOwned::to_owned).unwrap_or_default() != OsStr::new("osd") {
            return Err(anyhow!("FPV.WTF OSD files should have the .osd extension"))
        }
        Ok(())
    }
}