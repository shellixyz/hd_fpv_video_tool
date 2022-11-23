use std::path::PathBuf;

use clap::Args;
use getset::{Getters, CopyGetters};

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

    #[clap(short = 'o', long, value_parser, value_name = "frames", allow_negative_numbers(true), default_value_t = 0)]
    #[getset(skip)]
    #[getset(get_copy = "pub")]
    frame_shift: i32,

    /// path to FPV.WTF .osd file
    osd_file: PathBuf,

}