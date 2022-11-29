
pub use crate::{
    cli::{
        transcode_video_args::TranscodeVideoArgs,
        generate_overlay_args::GenerateOverlayArgs,
        start_end_args::StartEndArgs,
        transcode_video_args::TranscodeVideoOSDArgs,
    },
    file,
    osd::{
        Dimensions as OSDDimensions,
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
            OverlayVideoCodec,
        },
        region::{
            Region as OSDRegion,
        },
        coordinates::{
            Coordinate as OSDCoordinate,
            Coordinates as OSDCoordinates,
            FormatError as OSDCoordinatesFormatError,
        }
    },
    log_level::LogLevel,
    video::{
        self,
        AudioFixType as VideoAudioFixType,
        probe::Error as VideoProbingError,
    },
};

pub use hd_fpv_osd_font_tool::{
    dimensions::{
        Dimensions as GenericDimensions,
        FormatError as GenericDimensionsFormatError,
    },
};