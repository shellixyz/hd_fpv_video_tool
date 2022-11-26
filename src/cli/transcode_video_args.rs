
use std::path::{PathBuf, Path};

use clap::Args;
use getset::{Getters, CopyGetters};
use thiserror::Error;

use crate::{osd::{overlay::scaling::OSDScalingArgs, dji::file::find_associated_to_video_file}, prelude::VideoAudioFixType};

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
    osd_file: Option<PathBuf>,
}

#[derive(Debug, Error)]
#[error("args error: requested OSD but no file provided nor found")]
pub struct RequestedOSDButNoFileProvidedNorFound;

impl TranscodeVideoOSDArgs {
    pub fn osd_file_path<P: AsRef<Path>>(&self, video_file_path: P) -> Result<Option<PathBuf>, RequestedOSDButNoFileProvidedNorFound> {
        let osd_file_path = match (self.osd, &self.osd_file) {
            (true, None) => Some(find_associated_to_video_file(video_file_path).ok_or(RequestedOSDButNoFileProvidedNorFound)?),
            (_, Some(osd_file_path)) => Some(osd_file_path.clone()),
            (false, None) => None,
        };
        Ok(osd_file_path)
    }
}

#[derive(Args, Getters, CopyGetters)]
#[getset(get = "pub")]
pub struct TranscodeVideoArgs {
    /// fix DJI AU audio: fix sync + volume
    #[clap(short, long, value_parser)]
    #[getset(skip)]
    #[getset(get_copy = "pub")]
    fix_audio: bool,

    /// fix DJI AU audio volume
    #[clap(short, long, value_parser, conflicts_with("fix_audio"))]
    #[getset(skip)]
    #[getset(get_copy = "pub")]
    fix_audio_volume: bool,

    /// fix DJI AU audio sync
    #[clap(short, long, value_parser, conflicts_with("fix_audio"))]
    #[getset(skip)]
    #[getset(get_copy = "pub")]
    fix_audio_sync: bool,

    /// video encoder to use
    #[clap(short, long, value_parser, default_value = "libx265")]
    video_encoder: String,

    /// max bitrate
    #[clap(short, long, value_parser, default_value = "25M")]
    video_bitrate: String,

    /// video encoder to use
    #[clap(short, long, value_parser, default_value = "libx265")]
    audio_encoder: String,

    /// max bitrate
    #[clap(short, long, value_parser, default_value = "25M")]
    audio_bitrate: String,

    /// constant quality setting
    #[clap(short, long, value_parser, default_value_t = 30)]
    #[getset(skip)]
    #[getset(get_copy = "pub")]
    video_crf: u8,

    #[clap(flatten)]
    start_end: StartEndArgs,

    /// input video file path
    input_video_file: PathBuf,

    /// output video file path
    output_video_file: PathBuf,

    /// overwrite output file if it exists
    #[clap(short = 'y', long, value_parser)]
    #[getset(skip)]
    #[getset(get_copy = "pub")]
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
