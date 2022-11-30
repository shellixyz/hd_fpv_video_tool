
use std::str::FromStr;

use getset::CopyGetters;
use thiserror::Error;
use lazy_static::lazy_static;
use regex::Regex;


#[derive(Debug, Error)]
#[error("invalid margins format: {0}")]
pub struct InvalidMarginsFormatError(String);

#[derive(Debug, Clone, Copy, CopyGetters)]
#[getset(get_copy = "pub")]
pub struct Margins {
    horizontal: u32,
    vertical: u32,
}

impl FromStr for Margins {
    type Err = InvalidMarginsFormatError;

    fn from_str(margins_str: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref MARGINS_RE: Regex = Regex::new(r"\A(?P<horiz>\d{1,3}):(?P<vert>\d{1,3})\z").unwrap();
        }
        match MARGINS_RE.captures(margins_str) {
            Some(captures) => {
                let horizontal = captures.name("horiz").unwrap().as_str().parse().unwrap();
                let vertical = captures.name("vert").unwrap().as_str().parse().unwrap();
                Ok(Self { horizontal, vertical })
            },
            None => Err(InvalidMarginsFormatError(margins_str.to_owned())),
        }
    }
}