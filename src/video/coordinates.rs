
use std::{str::FromStr, ops::RangeInclusive};

use derive_more::From;
use getset::CopyGetters;
use regex::Regex;
use thiserror::Error;
use lazy_static::lazy_static;

use super::region::Region;


pub type Coordinate = u16;
pub type SignedCoordinate = i16;

#[derive(Debug, Clone, CopyGetters, From)]
#[getset(get_copy = "pub")]
pub struct SignedCoordinates {
    pub x: SignedCoordinate,
    pub y: SignedCoordinate,
}

impl SignedCoordinates {
    pub fn new(x: SignedCoordinate, y: SignedCoordinate) -> Self { Self { x, y } }
}

impl From<Coordinates> for SignedCoordinates {
    fn from(coordinates: Coordinates) -> Self {
        Self::new(
            SignedCoordinate::try_from(coordinates.x).unwrap(),
            SignedCoordinate::try_from(coordinates.y).unwrap(),
        )
    }
}

#[derive(Debug, Error)]
#[error("invalid screen coordinates format: {0}")]
pub struct FormatError(String);

#[derive(Debug, Clone, CopyGetters, From)]
#[getset(get_copy = "pub")]
pub struct Coordinates {
    pub x: Coordinate,
    pub y: Coordinate,
}

impl Coordinates {
    pub fn new(x: Coordinate, y: Coordinate) -> Self { Self { x, y } }
}

impl From<SignedCoordinates> for Coordinates {
    fn from(coordinates: SignedCoordinates) -> Self {
        Self::new(
            Coordinate::try_from(coordinates.x).unwrap(),
            Coordinate::try_from(coordinates.y).unwrap(),
        )
    }
}

impl FromStr for Coordinates {
    type Err = FormatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        lazy_static! { static ref ORIGIN_RE: Regex = Regex::new(r"\A(?P<x>\d{1,4}),(?P<y>\d{1,4})\z").unwrap(); }
        match ORIGIN_RE.captures(s) {
            Some(captures) => {
                let x = captures.name("x").unwrap().as_str().parse().unwrap();
                let y = captures.name("y").unwrap().as_str().parse().unwrap();
                Ok(Self { x, y })
            },
            None => Err(FormatError(s.to_owned())),
        }
    }
}

pub struct SignedRange {
    x_range: RangeInclusive<SignedCoordinate>,
    y_range: RangeInclusive<SignedCoordinate>,
}

impl SignedRange {

    pub fn new(x_range: RangeInclusive<SignedCoordinate>, y_range: RangeInclusive<SignedCoordinate>) -> Self {
        Self { x_range, y_range }
    }

    pub fn contains(&self, coordinates: Coordinates) -> bool {
        let coordinates = SignedCoordinates::from(coordinates);
        self.x_range.contains(&coordinates.x) && self.y_range.contains(&coordinates.y)
    }

}

impl From<&Region> for SignedRange {
    fn from(region: &Region) -> Self {
        let tlc = region.top_left_corner();
        let brc = region.bottom_right_corner();
        Self::new(tlc.x ..= brc.x, tlc.y ..= brc.y)
    }
}
