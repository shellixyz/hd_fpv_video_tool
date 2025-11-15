use clap::Args;
use getset::CopyGetters;
use thiserror::Error;

use crate::video::timestamp::Timestamp;

#[derive(Args, CopyGetters, Clone)]
#[getset(get_copy = "pub")]
pub struct StartEndArgs {
	/// start timestamp
	#[clap(long, value_parser, value_name = "[HH:]MM:SS")]
	start: Option<Timestamp>,

	/// end timestamp
	#[clap(long, value_parser, value_name = "[HH:]MM:SS")]
	end: Option<Timestamp>,
}

#[derive(Debug, Error)]
#[error("`--start` timestamp >= `--end` timestamp")]
pub struct StartGreaterThanEndError;

impl StartEndArgs {
	/// Returns true if the start and end timestamps are valid (start < end) or if either is None.
	#[must_use]
	pub fn are_valid(&self) -> bool {
		if let (Some(start), Some(end)) = (self.start, self.end) {
			return start < end;
		}
		true
	}

	/// Validates the start and end timestamps.
	///
	/// # Errors
	/// - Returns `StartGreaterThanEndError` if the start timestamp is greater than or equal to the end timestamp.
	pub fn check_valid(&self) -> Result<(), StartGreaterThanEndError> {
		if !self.are_valid() {
			return Err(StartGreaterThanEndError);
		}
		Ok(())
	}
}

#[derive(Args, CopyGetters, Clone)]
#[getset(get_copy = "pub")]
pub struct CutVideoStartEndArgs {
	/// start timestamp
	#[clap(short, long, value_parser, value_name = "[HH:]MM:SS")]
	start: Option<Timestamp>,

	/// end timestamp
	#[clap(short, long, value_parser, value_name = "[HH:]MM:SS")]
	end: Option<Timestamp>,
}

impl CutVideoStartEndArgs {
	/// Returns true if the start and end timestamps are valid (start < end) or if either is None.
	#[must_use]
	pub fn are_valid(&self) -> bool {
		if let (Some(start), Some(end)) = (self.start, self.end) {
			return start < end;
		}
		true
	}

	/// Validates the start and end timestamps.
	///
	/// # Errors
	/// - Returns `StartGreaterThanEndError` if the start timestamp is greater than or equal to the end timestamp.
	pub fn check_valid(&self) -> Result<(), StartGreaterThanEndError> {
		if !self.are_valid() {
			return Err(StartGreaterThanEndError);
		}
		Ok(())
	}
}
