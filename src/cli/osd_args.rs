
use std::path::PathBuf;

use clap::Args;
use getset::{Getters, CopyGetters};


#[derive(Args, Getters, CopyGetters)]
pub struct OSDArgs {
    /// shift frames to sync OSD with video
    #[clap(short = 'o', long, value_parser, value_name = "frames", default_value_t = 0)]
    #[getset(get_copy = "pub")]
    frame_shift: i32,

    /// path to FPV.WTF .osd file
    #[getset(get = "pub")]
    osd_file: PathBuf,
}

impl OSDArgs {
    pub fn new(frame_shift: i32, osd_file: PathBuf) -> Self {
        Self { frame_shift, osd_file }
    }
}
