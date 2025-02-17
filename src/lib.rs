// #![forbid(unsafe_code)]

pub mod cli;
pub mod create_path;
pub mod ffmpeg;
pub mod file;
pub mod image;
pub mod log_level;
pub mod osd;
pub mod prelude;
pub mod process;
pub mod video;

pub trait AsBool {
	fn as_bool(&self) -> bool;
}
