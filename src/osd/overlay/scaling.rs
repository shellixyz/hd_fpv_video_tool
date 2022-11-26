
use clap::Args;
use derive_more::From;
use getset::CopyGetters;
use thiserror::Error;

use super::{
    margins::{
        margin_value_parser,
        InvalidMarginsFormatError,
        Margins,
    },
};

use crate::video::resolution::{
    target_resolution_value_parser,
    InvalidTargetResolutionError,
    Resolution as VideoResolution,
    TargetResolution,
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
    #[error(transparent)]
    InvalidMarginsFormatError(InvalidMarginsFormatError),
    #[error("invalid minimum coverage percentage value: {0}")]
    InvalidMinCoveragePercent(u8),
    #[error("scaling and no-scaling arguments are mutually exclusive")]
    IncompatibleArguments,
    #[error("need target video resolution when scaling requested")]
    NeedTargetVideoResolution,
    #[error(transparent)]
    InvalidResolutionFormat(InvalidTargetResolutionError)
}

#[derive(Args, CopyGetters)]
#[getset(get_copy = "pub")]
pub struct ScalingArgs {

    /// resolution used to decide what kind of tiles (SD/HD) would best fit and also whether scaling should be used when in auto scaling mode
    #[clap(short = 'r', long, value_parser = target_resolution_value_parser, value_names = TargetResolution::valid_list())]
    target_resolution: Option<TargetResolution>,

    /// force using scaling, default is automatic
    #[clap(short, long, value_parser)]
    scaling: bool,

    /// force disable scaling, default is automatic
    #[clap(short, long, value_parser)]
    no_scaling: bool,

    /// minimum margins to decide whether scaling should be used and how much to scale
    #[clap(long, value_parser = min_margins_value_parser, value_name = "horizontal:vertical", default_value = "20:20")]
    min_margins: Margins,

    /// minimum percentage of OSD coverage under which scaling will be used if --scaling/--no-scaling options are not provided
    #[clap(long, value_parser = clap::value_parser!(u8).range(1..=100), value_name = "percent", default_value = "90")]
    min_coverage: u8,
}

#[derive(Args, CopyGetters)]
#[getset(get_copy = "pub")]
pub struct OSDScalingArgs {

    /// force using scaling, default is automatic
    #[clap(short, long, value_parser)]
    osd_scaling: bool,

    /// force disable scaling, default is automatic
    #[clap(short, long, value_parser)]
    no_osd_scaling: bool,

    /// minimum margins to decide whether scaling should be used and how much to scale
    #[clap(long, value_parser = min_margins_value_parser, value_name = "horizontal:vertical", default_value = "20:20")]
    min_osd_margins: Margins,

    /// minimum percentage of OSD coverage under which scaling will be used if --scaling/--no-scaling options are not provided
    #[clap(long, value_parser = clap::value_parser!(u8).range(1..=100), value_name = "percent", default_value = "90")]
    min_osd_coverage: u8,
}

fn min_margins_value_parser(min_margins_str: &str) -> Result<Margins, InvalidMarginsFormatError> {
    margin_value_parser(min_margins_str)
}

impl TryFrom<&ScalingArgs> for Scaling {
    type Error = ScalingArgsError;

    fn try_from(args: &ScalingArgs) -> Result<Self, Self::Error> {
        Ok(match (args.scaling, args.no_scaling) {
            (true, true) => return Err(ScalingArgsError::IncompatibleArguments),
            (true, false) => {
                let target_resolution = args.target_resolution.ok_or(ScalingArgsError::NeedTargetVideoResolution)?;
                Scaling::Yes { target_resolution, min_margins: args.min_margins }
            },
            (false, true) => Scaling::No { target_resolution: args.target_resolution },
            (false, false) => {
                if let Some(target_resolution) = args.target_resolution {
                    let min_coverage = args.min_coverage as f64 / 100.0;
                    let min_resolution = VideoResolution::new(
                        (target_resolution.dimensions().width as f64 * min_coverage) as u32,
                        (target_resolution.dimensions().height as f64 * min_coverage) as u32
                    );
                    Scaling::Auto { target_resolution, min_margins: args.min_margins, min_resolution }
                } else {
                    Scaling::No { target_resolution: args.target_resolution }
                }
            },
        })
    }
}

impl Scaling {
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