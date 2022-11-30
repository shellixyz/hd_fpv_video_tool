
use std::path::Path;

use clap::Args;
use derive_more::From;
use getset::{CopyGetters, Getters};
use thiserror::Error;

use super::{
    margins::{
        Margins,
    },
};

use crate::video::{
    resolution::{
        Resolution as VideoResolution,
        TargetResolution,
    },
    probe::{
        probe as video_probe,
        Error as VideoProbeError,
    }
};

#[derive(Debug, Clone, Copy)]
pub enum Scaling {
    No {
        target_resolution: Option<TargetResolution>,
    },
    Yes {
        target_resolution: TargetResolution,
        min_margins: Margins,
    },
    Auto {
        target_resolution: TargetResolution,
        min_margins: Margins,
        min_resolution: VideoResolution,
    }
}

#[derive(Debug, Error, From)]
pub enum ScalingArgsError {
    #[error("invalid minimum coverage percentage value: {0}")]
    InvalidMinCoveragePercent(u8),
    #[error("scaling and no-scaling arguments are mutually exclusive")]
    IncompatibleArguments,
    #[error("need target video resolution when scaling requested")]
    NeedTargetVideoResolution,
    #[error("both target video resolution and target video file provided")]
    BothTargetVideoResolutionAndFileProvided,
    #[error("failed to get video resolution from file: {0}")]
    VideoProbeError(VideoProbeError),
}

#[derive(Args, Getters, CopyGetters)]
#[getset(get_copy = "pub")]
pub struct ScalingArgs {

    /// resolution used to decide what kind of tiles (SD/HD) would best fit and also whether scaling should be used when in auto scaling mode
    #[clap(short = 'r', long, group("target_resolution_group"), value_parser, value_names = TargetResolution::valid_list())]
    target_resolution: Option<TargetResolution>,

    /// force using scaling, default is automatic
    #[clap(short, long, value_parser)]
    scaling: bool,

    /// force disable scaling, default is automatic
    #[clap(short, long, value_parser)]
    no_scaling: bool,

    /// minimum margins to decide whether scaling should be used and how much to scale
    #[clap(long, value_parser, value_name = "horizontal:vertical", default_value = "20:20")]
    min_margins: Margins,

    /// minimum percentage of OSD coverage under which scaling will be used if --scaling/--no-scaling options are not provided
    #[clap(long, value_parser = clap::value_parser!(u8).range(1..=100), value_name = "percent", default_value = "90")]
    min_coverage: u8,
}

#[derive(Args, CopyGetters)]
#[getset(get_copy = "pub")]
pub struct OSDScalingArgs {

    /// force using scaling, default is automatic
    #[clap(short = 's', long, value_parser)]
    osd_scaling: bool,

    /// force disable scaling, default is automatic
    #[clap(short, long, value_parser)]
    no_osd_scaling: bool,

    /// minimum margins to decide whether scaling should be used and how much to scale
    #[clap(long, value_parser, value_name = "horizontal:vertical", default_value = "20:20")]
    min_osd_margins: Margins,

    /// minimum percentage of OSD coverage under which scaling will be used if --scaling/--no-scaling options are not provided
    #[clap(long, value_parser = clap::value_parser!(u8).range(1..=100), value_name = "percent", default_value = "90")]
    min_osd_coverage: u8,
}

impl Scaling {

    pub fn try_from_scaling_args<P: AsRef<Path>>(args: &ScalingArgs, target_video_file: &Option<P>) -> Result<Self, ScalingArgsError> {
        let target_resolution = match (args.target_resolution, target_video_file) {
            (Some(target_resolution), None) => Some(target_resolution),
            (None, Some(video_file)) => {
                let probe_result = video_probe(video_file)?;
                Some(TargetResolution::from(probe_result.resolution()))
            }
            (None, None) => None,
            (Some(_), Some(_)) => return Err(ScalingArgsError::BothTargetVideoResolutionAndFileProvided)
        };

        Ok(match (args.scaling, args.no_scaling) {
            (true, true) => return Err(ScalingArgsError::IncompatibleArguments),
            (true, false) => {
                let target_resolution = target_resolution.ok_or(ScalingArgsError::NeedTargetVideoResolution)?;
                Scaling::Yes { target_resolution, min_margins: args.min_margins }
            },
            (false, true) => Scaling::No { target_resolution },
            (false, false) => {
                match target_resolution {
                    Some(target_resolution) => {
                    let min_coverage = args.min_coverage as f64 / 100.0;
                    let min_resolution = VideoResolution::new(
                        (target_resolution.dimensions().width as f64 * min_coverage) as u32,
                        (target_resolution.dimensions().height as f64 * min_coverage) as u32
                    );
                    Scaling::Auto { target_resolution, min_margins: args.min_margins, min_resolution }
                    },
                    None => Scaling::No { target_resolution }
                }
            },
        })
    }

    pub fn try_from_osd_args(args: &OSDScalingArgs, video_resolution: VideoResolution) -> Result<Self, ScalingArgsError> {
        Ok(match (args.osd_scaling, args.no_osd_scaling) {
            (true, true) => return Err(ScalingArgsError::IncompatibleArguments),
            (true, false) => Scaling::Yes { target_resolution: TargetResolution::Custom(video_resolution), min_margins: args.min_osd_margins },
            (false, true) => Scaling::No { target_resolution: Some(TargetResolution::Custom(video_resolution)) },
            (false, false) => {
                let target_resolution = TargetResolution::Custom(video_resolution);
                let min_coverage = args.min_osd_coverage as f64 / 100.0;
                let min_resolution = VideoResolution::new(
                    (target_resolution.dimensions().width as f64 * min_coverage) as u32,
                    (target_resolution.dimensions().height as f64 * min_coverage) as u32
                );
                Scaling::Auto { target_resolution, min_margins: args.min_osd_margins, min_resolution }
            },
        })
    }

}