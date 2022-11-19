
use std::fmt::Display;

use strum::{EnumIter, IntoEnumIterator};
use lazy_static::lazy_static;
use regex::Regex;

use hd_fpv_osd_font_tool::dimensions::Dimensions as GenericDimensions;
use thiserror::Error;


pub type VideoResolution = GenericDimensions<u32>;
pub type Resolution = GenericDimensions<u32>;

#[derive(Debug, Clone, Copy, EnumIter)]
pub enum StandardResolution {
    Tr720p,
    Tr720p4By3,
    Tr1080p,
    Tr1080p4by3,
}

impl Display for StandardResolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use StandardResolution::*;
        let value_str = match self {
             Tr720p => "720p",
             Tr720p4By3 => "720p4:3",
             Tr1080p => "1080p",
             Tr1080p4by3 => "1080p4:3",
        };
        f.write_str(value_str)
    }
}

impl StandardResolution {
    pub fn list() -> String {
        Self::iter().map(|std_res|
            std_res.to_string()
        ).collect::<Vec<_>>().join(", ")
    }

    pub fn dimensions(&self) -> VideoResolution {
        use StandardResolution::*;
        match self {
            Tr720p => VideoResolution::new(1280, 720),
            Tr720p4By3 => VideoResolution::new(960, 720),
            Tr1080p => VideoResolution::new(1920, 1080),
            Tr1080p4by3 => VideoResolution::new(1440, 1080),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TargetResolution {
    Standard(StandardResolution),
    Custom(VideoResolution),
}

impl TargetResolution {
    pub fn dimensions(&self) -> VideoResolution {
        use TargetResolution::*;
        match self {
            Standard(std_res) => std_res.dimensions(),
            Custom(resolution) => *resolution,
        }
    }

    pub fn valid_list() -> String {
        [StandardResolution::list(), "<width>x<height>".to_owned()].join(", ")
    }
}

#[derive(Debug, Error)]
#[error("invalid target resolution `{given}`, valid resolutions are: {valid}")]
pub struct InvalidTargetResolutionError {
    given: String,
    valid: String
}

impl TryFrom<&str> for TargetResolution {
    type Error = InvalidTargetResolutionError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        use TargetResolution::*;
        let resolution = match value {
            "720p" => Standard(StandardResolution::Tr720p),
            "720p4:3" => Standard(StandardResolution::Tr1080p4by3),
            "1080p" => Standard(StandardResolution::Tr1080p),
            "1080p4:3" => Standard(StandardResolution::Tr1080p4by3),
            custom_res_str => {
                lazy_static! {
                    static ref RES_RE: Regex = Regex::new(r"\A(?P<width>\d{1,5})x(?P<height>\d{1,5})\z").unwrap();
                }
                match RES_RE.captures(custom_res_str) {
                    Some(captures) => {
                        let width = captures.name("width").unwrap().as_str().parse().unwrap();
                        let height = captures.name("height").unwrap().as_str().parse().unwrap();
                        Custom(VideoResolution::new(width, height))
                    },
                    None =>
                        return Err(InvalidTargetResolutionError {
                            given: custom_res_str.to_owned(),
                            valid: Self::valid_list()
                        }),
                }
            }
        };
        Ok(resolution)
    }
}
