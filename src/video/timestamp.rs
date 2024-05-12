
use std::{fmt::Display, str::FromStr};

use derive_more::Constructor;
use ffmpeg_next::Rational;
use getset::{CopyGetters, Setters};
use regex::Regex;
use thiserror::Error;
use lazy_static::lazy_static;


#[derive(Debug, CopyGetters, Setters, Constructor, Clone, Copy, Default, PartialEq, Eq)]
#[getset(get_copy = "pub", set = "pub")]
pub struct Timestamp {
    hours: u16,
    minutes: u8,
    seconds: u8,
}

impl Timestamp {

    pub fn total_seconds(&self) -> u32 {
        self.hours as u32 * 3600 + self.minutes as u32 * 60 + self.seconds as u32
    }

    pub fn to_ffmpeg_position(&self) -> String {
        format!("{}:{}:{}", self.hours, self.minutes, self.seconds)
    }

    pub fn frame_count(&self, fps: Rational) -> u64 {
        let frame_exact = fps * ffmpeg_next::Rational::new(self.total_seconds() as i32, 1);
        (frame_exact.numerator() as f64 / frame_exact.denominator() as f64).round() as u64
    }

    pub fn overlay_frame_count(&self) -> u32 {
        u32::try_from(self.frame_count(Rational::from((60, 1)))).unwrap()
    }

    pub fn overlay_frame_index(&self) -> u32 {
        let frame_count = self.overlay_frame_count();
        if frame_count < 1 {
            return frame_count;
        }
        frame_count - 1
    }

    pub fn interval_frames(start_timestamp: &Self, end_timestamp: &Self, fps: Rational) -> u64 {
        let interval_seconds = end_timestamp.total_seconds() as i32 - start_timestamp.total_seconds() as i32;
        if interval_seconds < 0 { return 0 }
        let frames_exact = fps * ffmpeg_next::Rational::new(interval_seconds, 1);
        (frames_exact.numerator() as f64 / frames_exact.denominator() as f64).round() as u64
    }

}

impl Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.hours > 0 { write!(f, "{}:", self.hours)? }
        write!(f, "{}:{}", self.minutes, self.seconds)
    }
}

impl PartialOrd for Timestamp {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.total_seconds().cmp(&other.total_seconds()))
    }
}

impl Ord for Timestamp {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.total_seconds().cmp(&other.total_seconds())
    }
}

#[derive(Debug, Error)]
#[error("invalid timestamp: {0}")]
pub struct TimestampFormatError(String);

impl FromStr for Timestamp {
    type Err = TimestampFormatError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref TIMESTAMP_RE: Regex = Regex::new(r"\A(?:(?P<hours>\d{1,3}):)?(?P<minutes>\d{1,2}):(?P<seconds>\d{1,2})\z").unwrap();
        }
        Ok(match TIMESTAMP_RE.captures(value) {
            Some(captures) => {
                let hours = captures.name("hours").map(|hours_match| hours_match.as_str().parse().unwrap()).unwrap_or(0);
                let minutes = captures.name("minutes").unwrap().as_str().parse().unwrap();
                let seconds = captures.name("seconds").unwrap().as_str().parse().unwrap();
                Timestamp::new(hours, minutes, seconds)
            },
            None => return Err(TimestampFormatError(value.to_owned())),
        })
    }
}

pub trait StartEndOverlayFrameIndex {
    fn start_overlay_frame_count(&self) -> u32;
    fn end_overlay_frame_index(&self) -> Option<u32>;
}

impl StartEndOverlayFrameIndex for Option<Timestamp> {

    fn start_overlay_frame_count(&self) -> u32 {
        match self {
            Some(start) => start.overlay_frame_count(),
            None => 0,
        }
    }

    fn end_overlay_frame_index(&self) -> Option<u32> {
        self.as_ref().map(|end| end.overlay_frame_index())
    }

}