use std::{path::{PathBuf, Path}, ffi::CStr};

use ffmpeg_next as ffmpeg;

use ffmpeg::Rational;
use getset::{CopyGetters, Getters};
use thiserror::Error;

use super::resolution::Resolution;


#[derive(Debug, Error)]
#[error("failed to probe video file {file_path}: {error}")]
pub struct Error {
    file_path: PathBuf,
    error: String,
}

impl Error {
    pub fn new<P: AsRef<Path>>(file_path: P, error: &str) -> Self {
        Self { file_path: file_path.as_ref().to_path_buf(), error: error.to_owned() }
    }
}

#[derive(Debug, Clone, CopyGetters, Getters)]
#[getset(get_copy = "pub")]
pub struct Result {
    frame_count: u64,
    frame_rate: Rational,
    has_audio: bool,
    resolution: Resolution,

    #[getset(skip)] #[getset(get = "pub")]
    video_codec: Option<String>,
}

pub fn probe<P: AsRef<Path>>(video_file: P) -> std::result::Result<Result, Error> {
    ffmpeg::init().unwrap();
    ffmpeg::log::set_level(ffmpeg::log::Level::Quiet);

    let input = ffmpeg::format::input(&video_file)
        .map_err(|_| Error::new(&video_file, "failed to open video file"))?;

    let has_audio = input.streams().best(ffmpeg::media::Type::Audio).is_some();

    let video_stream = input.streams().best(ffmpeg::media::Type::Video)
        .ok_or_else(|| Error::new(&video_file, "cannot find video stream"))?;

    let video_stream_parameters = video_stream.parameters();
    let (width, height) = unsafe { ((*video_stream_parameters.as_ptr()).width, (*video_stream_parameters.as_ptr()).height) };
    let resolution = Resolution::new(width as u32, height as u32);

    let video_codec = unsafe {
        let av_codec_id = ffmpeg::ffi::avcodec_descriptor_get((*video_stream_parameters.as_ptr()).codec_id);
        if av_codec_id.is_null() {
            None
        } else {
            match (*av_codec_id).name {
                name_ptr if name_ptr.is_null() => None,
                name_ptr => Some(String::from_utf8_lossy(CStr::from_ptr(name_ptr).to_bytes()).to_string())
            }
        }
    };

    let frame_rate = video_stream.rate();

    let frame_count = u64::try_from(video_stream.frames())
        .map_err(|_| Error::new(&video_file, "failed to get frame count"))?;

    Ok(Result { frame_count, frame_rate, has_audio, resolution, video_codec })
}
