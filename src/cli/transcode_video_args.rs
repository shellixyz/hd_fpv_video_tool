
use std::path::PathBuf;

use clap::Args;
use getset::{Getters, CopyGetters};

use crate::{osd::overlay::scaling::OSDScalingArgs, prelude::VideoAudioFixType};

use super::{font_options::OSDFontOptions, start_end_args::StartEndArgs};


#[derive(Args, Getters, CopyGetters)]
pub struct TranscodeVideoOSDArgs {

    /// burn OSD onto video, try to find the OSD file automatically
    #[clap(long, value_parser)]
    #[getset(get_copy = "pub")]
    osd: bool,

    #[clap(flatten)]
    #[getset(get = "pub")]
    osd_scaling_args: OSDScalingArgs,

    #[clap(flatten)]
    #[getset(get = "pub")]
    osd_font_options: OSDFontOptions,

    /// shift frames to sync OSD with video
    #[clap(short = 'o', long, value_parser, value_name = "frames", default_value_t = 0)]
    #[getset(get_copy = "pub")]
    osd_frame_shift: i32,

    /// path to FPV.WTF .osd file to use to generate OSD frames to burn onto video
    #[clap(long, value_parser, value_name = "OSD file path")]
    #[getset(get = "pub")]
    osd_file: Option<PathBuf>,
}

#[derive(Args, Getters, CopyGetters)]
#[getset(get_copy = "pub")]
pub struct TranscodeVideoArgs {
    /// fix DJI AU audio: fix sync + volume
    #[clap(short, long, value_parser)]
    fix_audio: bool,

    /// fix DJI AU audio volume
    #[clap(short, long, value_parser, conflicts_with("fix_audio"))]
    fix_audio_volume: bool,

    /// fix DJI AU audio sync
    #[clap(short, long, value_parser, conflicts_with("fix_audio"))]
    fix_audio_sync: bool,

    /// video encoder to use
    #[clap(short, long, value_parser, default_value = "libx265")]
    #[getset(skip)]
    #[getset(get = "pub")]
    encoder: String,

    /// max bitrate
    #[clap(short, long, value_parser, default_value = "25M")]
    #[getset(skip)]
    #[getset(get = "pub")]
    bitrate: String,

    /// constant quality setting
    #[clap(short, long, value_parser, default_value_t = 30)]
    crf: u8,

    // /// start timestamp
    // #[clap(long, value_parser = timestamp_value_parser, value_name = "[HH:]MM:SS", conflicts_with("fix_audio"), conflicts_with("fix_audio_sync"))]
    // start: Option<Timestamp>,

    // /// end timestamp
    // #[clap(long, value_parser = timestamp_value_parser, value_name = "[HH:]MM:SS")]
    // end: Option<Timestamp>,

    #[clap(flatten)]
    #[getset(skip)]
    #[getset(get = "pub")]
    start_end: StartEndArgs,

    /// input video file path
    #[getset(skip)]
    #[getset(get = "pub")]
    input_video_file: PathBuf,

    /// output video file path
    #[getset(skip)]
    #[getset(get = "pub")]
    output_video_file: PathBuf,

    /// overwrite output file if it exists
    #[clap(short = 'y', long, value_parser)]
    overwrite: bool,
}

impl TranscodeVideoArgs {
    pub fn video_audio_fix(&self) -> Option<VideoAudioFixType> {
        use VideoAudioFixType::*;
        match (self.fix_audio, self.fix_audio_sync, self.fix_audio_volume) {
            (true, _, _) | (false, true, true) => Some(SyncAndVolume),
            (false, true, false) => Some(Sync),
            (false, false, true) => Some(Volume),
            (false, false, false) => None,
        }
    }
}
