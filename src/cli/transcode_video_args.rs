
use std::path::{PathBuf, Path};

use clap::Args;
use getset::{Getters, CopyGetters};
use thiserror::Error;

use crate::{osd::{self, overlay::scaling::OSDScalingArgs, file::find_associated_to_video_file}, video};

use super::{font_options::OSDFontOptions, start_end_args::StartEndArgs, generate_overlay_args};


#[derive(Args, Getters, CopyGetters)]
pub struct TranscodeVideoOSDArgs {

    /// burn OSD onto video, try to find the OSD file automatically.
    ///
    /// First tries finding a file with the name <basename of the video file>.osd then if it does not exist
    /// tries finding a file with same DJI prefix as the video file with G instead of U if it is starting with DJIU. Examples:{n}
    /// DJIG0000.mp4 => DJIG0000.osd{n}
    /// DJIG0000_something.mp4 => DJIG0000.osd{n}
    /// DJIU0000.mp4 => DJIG0000.osd{n}
    /// DJIU0000_something.mp4 => DJIG0000.osd{n}
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
    #[clap(short = 'o', long, value_parser, allow_negative_numbers(true), value_name = "frames")]
    #[getset(get_copy = "pub")]
    osd_frame_shift: Option<i32>,

    /// hide rectangular regions from the OSD
    ///
    /// The parameter is a `;` separated list of regions.{n}
    /// The format for a region is: <left_x>,<top_y>[:<width>x<height>]{n}
    /// If the size is not specified it will default to 1x1
    #[clap(long, value_parser, value_delimiter = ';', value_name = "REGIONS")]
    #[getset(get = "pub")]
    osd_hide_regions: Vec<osd::Region>,

    /// hide items from the OSD
    #[clap(long, value_parser, value_delimiter = ',', value_name = "OSD_ITEM_NAMES", help = generate_overlay_args::osd_hide_items_arg_help())]
    #[getset(get = "pub")]
    osd_hide_items: Vec<String>,

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
    #[clap(short = 'v', long, value_parser, conflicts_with("fix_audio"))]
    #[getset(skip)]
    #[getset(get_copy = "pub")]
    fix_audio_volume: bool,

    /// fix DJI AU audio sync
    #[clap(short = 'u', long, value_parser, conflicts_with("fix_audio"))]
    #[getset(skip)]
    #[getset(get_copy = "pub")]
    fix_audio_sync: bool,

    /// video encoder to use
    ///
    /// This value is directly passed to the `-c:v` FFMpeg argument.{n}
    /// Run `ffmpeg -encoders` for a list of available encoders
    #[clap(long, value_parser, default_value = "libx265")]
    video_encoder: String,

    /// video max bitrate
    #[clap(long, value_parser, default_value = "25M")]
    video_bitrate: String,

    /// video constant quality setting
    #[clap(long, value_parser, default_value_t = 25)]
    #[getset(skip)]
    #[getset(get_copy = "pub")]
    video_crf: u8,

    /// remove video defects
    ///
    /// uses the FFMpeg delogo filter to remove small video defects
    ///
    /// The parameter is a `;` separated list of regions.{n}
    /// The format for a region is: <left_x>,<top_y>[:<width>x<height>]{n}
    /// If the size is not specified it will default to 1x1
    #[clap(long, value_parser, value_delimiter = ';', value_name = "REGIONS")]
    remove_video_defects: Vec<video::Region>,

    /// audio encoder to use
    ///
    /// This value is directly passed to the `-c:a` FFMpeg argument.{n}
    /// Run `ffmpeg -encoders` for a list of available encoders
    #[clap(long, value_parser, default_value = "aac")]
    audio_encoder: String,

    /// max audio bitrate
    #[clap(long, value_parser, default_value = "93k")]
    audio_bitrate: String,

    #[clap(flatten)]
    start_end: StartEndArgs,

    /// input video file path
    input_video_file: PathBuf,

    /// output video file path
    #[getset(skip)]
    output_video_file: Option<PathBuf>,

    /// overwrite output file if it exists
    #[clap(short = 'y', long, value_parser)]
    #[getset(skip)]
    #[getset(get_copy = "pub")]
    overwrite: bool,
}

#[derive(Debug, Error)]
pub enum OutputVideoFileError {
    #[error("input has no file name")]
    InputHasNoFileName,
    #[error("input has no extension")]
    InputHasNoExtension,
}

impl TranscodeVideoArgs {

    pub fn video_audio_fix(&self) -> Option<video::AudioFixType> {
        use video::AudioFixType::*;
        match (self.fix_audio, self.fix_audio_sync, self.fix_audio_volume) {
            (true, _, _) | (false, true, true) => Some(SyncAndVolume),
            (false, true, false) => Some(Sync),
            (false, false, true) => Some(Volume),
            (false, false, false) => None,
        }
    }

    pub fn output_video_file_provided(&self) -> bool {
        self.output_video_file.is_some()
    }

    pub fn output_video_file(&self, with_osd: bool) -> Result<PathBuf, OutputVideoFileError> {
        Ok(match &self.output_video_file {
            Some(output_video_file) => output_video_file.clone(),
            None => {
                let mut output_file_stem = Path::new(self.input_video_file.file_stem().ok_or(OutputVideoFileError::InputHasNoFileName)?).as_os_str().to_os_string();
                let suffix = if with_osd { "_with_osd" } else { "_transcoded" };
                output_file_stem.push(suffix);
                let input_file_extension = self.input_video_file.extension().ok_or(OutputVideoFileError::InputHasNoExtension)?;
                self.input_video_file.with_file_name(output_file_stem).with_extension(input_file_extension)
            }
        })
    }

}
