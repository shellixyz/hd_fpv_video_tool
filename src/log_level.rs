
use clap::ValueEnum;
use strum::Display;

#[derive(Copy, Clone, Display, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum LogLevel {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}