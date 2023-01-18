
use thiserror::Error;

use crate::video::resolution::Resolution as VideoResolution;

use super::Kind;

pub mod file;
pub mod font_dir;


pub const AU_OSD_FRAME_SHIFT: i32 = -36;

#[derive(Debug, Error)]
#[error("video resolution {video_resolution} too small to fit {osd_kind} kind OSD")]
pub struct VideoResolutionTooSmallError {
    pub osd_kind: Kind,
    pub video_resolution: VideoResolution
}

pub mod dimensions {
    use crate::osd::Dimensions;
    pub const SD: Dimensions = Dimensions::new(30, 15);
    pub const FAKE_HD: Dimensions = Dimensions::new(60, 22);
    pub const HD: Dimensions = Dimensions::new(50, 18);
}
