
pub use crate::{
    osd::{
        dji::{
            font_dir::FontDir,
            file::{
                OpenError as OSDFileOpenError,
                Reader as OSDFileReader,
            },
        },
        overlay::{
            DrawFrameOverlayError,
            Generator as OverlayGenerator,
            SaveFramesToDirError,
            scaling::{
                Scaling,
                ScalingArgs,
            },
        },
    },
    log_level::LogLevel,
    video::{
        fix_dji_air_unit_video_file_audio,
        transcode_video,
        transcode_video_burn_osd,
        TranscodeArgs,
        AudioFixType as VideoAudioFixType,
    },
};