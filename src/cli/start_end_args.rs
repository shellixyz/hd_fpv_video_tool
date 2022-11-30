use clap::Args;
use getset::CopyGetters;
use thiserror::Error;

use crate::video::timestamp::Timestamp;


#[derive(Args, CopyGetters)]
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

    pub fn are_valid(&self) -> bool {
        if let (Some(start), Some(end)) = (self.start, self.end) {
            return start < end;
        }
        true
    }

    pub fn check_valid(&self) -> Result<(), StartGreaterThanEndError> {
        if ! self.are_valid() {
            return Err(StartGreaterThanEndError);
        }
        Ok(())
    }

}