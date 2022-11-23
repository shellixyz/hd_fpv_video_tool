
pub use crate::{
    cli::{
        transcode_video_args::TranscodeVideoArgs,
    },
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
        AudioFixType as VideoAudioFixType,
        probe::{
            probe as video_probe,
            Error as VideoProbingError,
        }
    },
};