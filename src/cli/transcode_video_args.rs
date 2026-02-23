use std::{
	collections::BTreeMap,
	ffi::OsString,
	io::Error as IOError,
	path::{Path, PathBuf},
	str::FromStr,
};

use clap::Args;
use getset::{CopyGetters, Getters};
use serde::Deserialize;
use strum::IntoEnumIterator as _;
use thiserror::Error;

use super::{font_options::OSDFontOptions, generate_overlay_args, start_end_args::StartEndArgs};
use crate::{
	AsBool,
	ffmpeg::{self, VideoQuality},
	osd::{self, file::find_associated_to_video_file, overlay::scaling::OSDScalingArgs},
	prelude::OverlayVideoCodec,
	video::{self, HwAcceleratedEncoding, resolution::TargetResolution},
};

const DEFAULT_VIDEO_BITRATE: &str = "25M";
const TRANSCODE_PROFILE_DIGITAL_FPV: &str = "digital-fpv";
const TRANSCODE_PROFILE_ANALOG_FPV: &str = "analog-fpv";
const USER_CONFIG_FILE_HOME_RELATIVE_PATH: &str = ".config/hd_fpv_video_tool.toml";

impl FromStr for video::Codec {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		use video::Codec as C;
		Ok(match s.to_uppercase().as_str() {
			"AV1" => C::AV1,
			"H264" | "H.264" => C::H264,
			"H265" | "H.265" => C::H265,
			"VP8" => C::VP8,
			"VP9" => C::VP9,
			_ => return Err(format!("unknown video codec: {s}")),
		})
	}
}

impl video::Codec {
	pub fn default_video_quality(&self, hw_accel: &impl AsBool) -> ffmpeg::VideoQuality {
		#[allow(clippy::match_bool, clippy::match_same_arms)]
		match hw_accel.as_bool() {
			true => match self {
				video::Codec::AV1 => VideoQuality::GlobalQuality(120),
				video::Codec::H264 => VideoQuality::GlobalQuality(23), // to figure out
				video::Codec::H265 => VideoQuality::GlobalQuality(22),
				video::Codec::VP8 => VideoQuality::GlobalQuality(30), // to figure out
				video::Codec::VP9 => VideoQuality::GlobalQuality(30), // to figure out
			},
			false => match self {
				video::Codec::AV1 => VideoQuality::ConstantRateFactor(30), // to figure out
				video::Codec::H264 => VideoQuality::ConstantRateFactor(23), // to figure out
				video::Codec::H265 => VideoQuality::ConstantRateFactor(25),
				video::Codec::VP8 => VideoQuality::ConstantRateFactor(30), // to figure out
				video::Codec::VP9 => VideoQuality::ConstantRateFactor(30), // to figure out
			},
		}
	}
}

#[derive(Args, Getters, CopyGetters)]
pub struct TranscodeVideoOSDArgs {
	/// burn OSD onto video.
	///
	/// If --osd-file is not provided, the tool will try to find the OSD file automatically.
	/// First tries finding a file with the name <basename of the video file>.osd then if it does
	/// not exist tries finding a file with same DJI prefix as the video file with G instead of U
	/// if it is starting with DJIU. Examples:{n} DJIG0000.mp4 => DJIG0000.osd{n}
	/// `DJIG0000_something.mp4` => `DJIG0000.osd`{n}
	/// `DJIU0000.mp4` => `DJIG0000.osd`{n}
	/// `DJIU0000_something.mp4` => `DJIG0000.osd`{n}
	#[clap(long, value_parser)]
	#[getset(get_copy = "pub")]
	osd: bool,

	#[clap(flatten)]
	#[getset(get = "pub")]
	osd_scaling_args: OSDScalingArgs,

	#[clap(flatten)]
	#[getset(get = "pub")]
	osd_font_options: OSDFontOptions,

	/// shift frames to sync OSD with video
	#[clap(short = 'o', long, value_parser, allow_negative_numbers(true), value_name = "frames")]
	#[getset(get_copy = "pub")]
	osd_frame_shift: Option<i32>,

	/// hide rectangular regions from the OSD
	///
	/// The parameter is a `;` separated list of regions.{n}
	/// The format for a region is: <`left_x`>,<`top_y`>[:<`width`>x<`height`>]{n}
	/// If the size is not specified it will default to 1x1
	#[clap(long, value_parser, value_delimiter = ';', value_name = "REGIONS")]
	#[getset(get = "pub")]
	osd_hide_regions: Vec<osd::Region>,

	/// hide items from the OSD
	#[clap(long, value_parser, value_delimiter = ',', value_name = "OSD_ITEM_NAMES", help = generate_overlay_args::osd_hide_items_arg_help())]
	#[getset(get = "pub")]
	osd_hide_items: Vec<String>,

	/// generate OSD overlay video instead of burning it onto the video
	#[clap(short = 'O', long)]
	#[getset(get_copy = "pub")]
	osd_overlay_video: bool,

	#[clap(long, default_value = "vp8", requires = "osd_overlay_video")]
	#[getset(get_copy = "pub")]
	osd_overlay_video_codec: OverlayVideoCodec,

	/// path of the video file to generate
	#[clap(long, requires = "osd_overlay_video")]
	#[getset(get = "pub")]
	osd_overlay_video_file: Option<PathBuf>,

	/// path to FPV.WTF .osd file to use to generate OSD frames to burn onto video
	#[clap(short = 'F', long, value_parser, value_name = "OSD file path")]
	osd_file: Option<PathBuf>,
}

#[derive(Debug, Error)]
#[error("args error: requested OSD but no file provided nor found")]
pub struct RequestedOSDButNoFileProvidedNorFound;

impl TranscodeVideoOSDArgs {
	/// Returns the path to the OSD file to use for generating OSD frames.
	/// If no OSD file is provided and OSD is requested, tries to find the associated OSD file.
	///
	/// # Errors
	/// - Returns `RequestedOSDButNoFileProvidedNorFound` if OSD is requested but no file is provided nor found.
	pub fn osd_file_path<P: AsRef<Path>>(
		&self,
		video_file_path: P,
	) -> Result<Option<PathBuf>, RequestedOSDButNoFileProvidedNorFound> {
		let osd_file_path = match (self.osd || self.osd_overlay_video, &self.osd_file) {
			(true, None) => {
				Some(find_associated_to_video_file(video_file_path).ok_or(RequestedOSDButNoFileProvidedNorFound)?)
			},
			(_, Some(osd_file_path)) => Some(osd_file_path.clone()),
			(false, None) => None,
		};
		Ok(osd_file_path)
	}
}

#[derive(Args, Getters, CopyGetters)]
#[getset(get = "pub")]
#[allow(clippy::struct_excessive_bools)]
pub struct TranscodeVideoArgs {
	/// add audio stream to the output video
	///
	/// This is useful when the input video does not have an audio stream
	/// and you want to splice it with other videos that do have audio
	/// and you want to keep the audio from the other videos
	#[clap(short, long)]
	#[getset(skip)]
	#[getset(get_copy = "pub")]
	add_audio: bool,

	/// fix DJI AU audio: fix sync + volume
	#[clap(short, long, value_parser)]
	#[getset(skip)]
	#[getset(get_copy = "pub")]
	fix_audio: bool,

	/// fix DJI AU audio volume
	#[clap(short = 'v', long, value_parser, conflicts_with("fix_audio"))]
	#[getset(skip)]
	#[getset(get_copy = "pub")]
	fix_audio_volume: bool,

	/// fix DJI AU audio sync
	#[clap(short = 'u', long, value_parser, conflicts_with("fix_audio"))]
	#[getset(skip)]
	#[getset(get_copy = "pub")]
	fix_audio_sync: bool,

	#[cfg(feature = "hwaccel")]
	/// disable hardware acceleration
	#[clap(short = 'N', long, default_value_t = false)]
	#[getset(skip)]
	#[getset(get_copy = "pub")]
	no_hwaccel: bool,

	#[clap(short = 'V', long, help = transcode_video_args_video_codec_help())]
	#[getset(skip)]
	video_codec: Option<video::Codec>,

	#[clap(short = 'p', long, value_parser, help = transcode_video_args_profile_help(), value_name = "PROFILE")]
	#[getset(skip)]
	profile: Option<String>,

	/// video max bitrate
	#[clap(long, value_parser)]
	#[getset(skip)]
	video_bitrate: Option<String>,

	/// video constant quality setting
	#[clap(short = 'q', long)]
	#[getset(skip)]
	#[getset(get_copy = "pub")]
	video_quality: Option<u8>,

	/// [possible values: 720p, 720p4:3, 1080p, 1080p4:3, <width>x<height>]
	#[clap(short = 'r', long)]
	#[getset(skip)]
	#[getset(get_copy = "pub")]
	video_resolution: Option<TargetResolution>,

	/// remove video defects
	///
	/// uses the `FFMpeg` delogo filter to remove small video defects
	///
	/// The parameter is a `;` separated list of regions.{n}
	/// The format for a region is: <`left_x`>,<`top_y`>[:<`width`>x<`height`>]{n}
	/// If the size is not specified it will default to 1x1
	#[clap(long, value_parser, value_delimiter = ';', value_name = "REGIONS")]
	remove_video_defects: Vec<video::Region>,

	/// audio encoder to use
	///
	/// This value is directly passed to the `-c:a` `FFMpeg` argument.{n}
	/// Run `ffmpeg -encoders` for a list of available encoders
	#[clap(long, value_parser, default_value = "aac")]
	audio_encoder: String,

	/// max audio bitrate
	#[clap(long, value_parser, default_value = "93k")]
	audio_bitrate: String,

	#[clap(flatten)]
	start_end: StartEndArgs,

	/// process scheduling priority to give to `FFMpeg` from -20 to 19
	#[clap(short = 'P', long, value_parser = clap::value_parser!(i32).range(-20..=19), value_name = "PRIORITY")]
	ffmpeg_priority: Option<i32>,

	/// input video file path
	input_video_file: PathBuf,

	/// output video file path
	#[getset(skip)]
	output_video_file: Option<PathBuf>,

	/// overwrite output file if it exists
	#[clap(short = 'y', long, value_parser)]
	#[getset(skip)]
	#[getset(get_copy = "pub")]
	overwrite: bool,

	#[clap(short = 'S', long, value_parser)]
	#[getset(skip)]
	#[getset(get_copy = "pub")]
	speed: Option<f64>,
}

fn transcode_video_args_video_codec_help() -> String {
	let video_codecs = video::Codec::iter()
		.map(|video_codec| video_codec.to_string().to_uppercase())
		.collect::<Vec<_>>()
		.join(", ");
	format!("video codec to use. Possible values: {video_codecs}")
}

fn transcode_video_args_profile_help() -> String {
	format!(
		"transcode profile name. Built-in profiles: {TRANSCODE_PROFILE_DIGITAL_FPV}, {TRANSCODE_PROFILE_ANALOG_FPV}. \
		 Custom profiles can be defined in ~/{USER_CONFIG_FILE_HOME_RELATIVE_PATH}"
	)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum ProfileBitrate {
	String(String),
	Integer(i64),
}

impl From<ProfileBitrate> for String {
	fn from(profile_bitrate: ProfileBitrate) -> Self {
		match profile_bitrate {
			ProfileBitrate::String(value) => value,
			ProfileBitrate::Integer(value) => value.to_string(),
		}
	}
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct TranscodeCodecProfileToml {
	video_bitrate: Option<ProfileBitrate>,
	video_quality: Option<u8>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct TranscodeProfileToml {
	video_codec: Option<String>,
	default_video_bitrate: Option<ProfileBitrate>,
	default_video_quality: Option<u8>,
	h264: Option<TranscodeCodecProfileToml>,
	h265: Option<TranscodeCodecProfileToml>,
	av1: Option<TranscodeCodecProfileToml>,
	vp8: Option<TranscodeCodecProfileToml>,
	vp9: Option<TranscodeCodecProfileToml>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct TranscodeVideoConfigToml {
	profiles: BTreeMap<String, TranscodeProfileToml>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct HDVideoToolConfigToml {
	transcode_video: TranscodeVideoConfigToml,
}

#[derive(Debug, Clone, Default)]
struct TranscodeCodecProfile {
	video_bitrate: Option<String>,
	video_quality: Option<u8>,
}

impl From<TranscodeCodecProfileToml> for TranscodeCodecProfile {
	fn from(value: TranscodeCodecProfileToml) -> Self {
		Self {
			video_bitrate: value.video_bitrate.map(String::from),
			video_quality: value.video_quality,
		}
	}
}

#[derive(Debug, Clone, Default)]
struct TranscodeProfile {
	video_codec: Option<video::Codec>,
	default_video_bitrate: Option<String>,
	default_video_quality: Option<u8>,
	h264: TranscodeCodecProfile,
	h265: TranscodeCodecProfile,
	av1: TranscodeCodecProfile,
	vp8: TranscodeCodecProfile,
	vp9: TranscodeCodecProfile,
}

impl TranscodeProfile {
	fn codec_profile(&self, video_codec: video::Codec) -> &TranscodeCodecProfile {
		match video_codec {
			video::Codec::AV1 => &self.av1,
			video::Codec::H264 => &self.h264,
			video::Codec::H265 => &self.h265,
			video::Codec::VP8 => &self.vp8,
			video::Codec::VP9 => &self.vp9,
		}
	}

	fn from_toml(profile_name: &str, profile_toml: TranscodeProfileToml) -> Result<Self, TranscodeVideoProfileError> {
		let video_codec = match profile_toml.video_codec {
			Some(video_codec_str) => Some(video::Codec::from_str(&video_codec_str).map_err(|_| {
				TranscodeVideoProfileError::InvalidCodec {
					profile_name: profile_name.to_owned(),
					video_codec: video_codec_str,
				}
			})?),
			None => None,
		};
		Ok(Self {
			video_codec,
			default_video_bitrate: profile_toml.default_video_bitrate.map(String::from),
			default_video_quality: profile_toml.default_video_quality,
			h264: profile_toml.h264.unwrap_or_default().into(),
			h265: profile_toml.h265.unwrap_or_default().into(),
			av1: profile_toml.av1.unwrap_or_default().into(),
			vp8: profile_toml.vp8.unwrap_or_default().into(),
			vp9: profile_toml.vp9.unwrap_or_default().into(),
		})
	}
}

#[derive(Debug, Error)]
pub enum TranscodeVideoProfileError {
	#[error("unable to locate home directory to read transcode profiles")]
	UnableToLocateHomeDirectory,
	#[error("failed to read transcode profiles from `{path}`: {error}")]
	ConfigReadError { path: PathBuf, error: IOError },
	#[error("failed to parse transcode profiles from `{path}`: {error}")]
	ConfigParseError { path: PathBuf, error: toml::de::Error },
	#[error("profile `{profile_name}` not found")]
	UnknownProfile { profile_name: String },
	#[error("invalid video codec `{video_codec}` in profile `{profile_name}`")]
	InvalidCodec { profile_name: String, video_codec: String },
}

fn codec_profile_summary(codec_profile: &TranscodeCodecProfile) -> Option<String> {
	if codec_profile.video_bitrate.is_none() && codec_profile.video_quality.is_none() {
		return None;
	}
	let bitrate = codec_profile.video_bitrate.clone().unwrap_or_else(|| "-".to_owned());
	let quality = codec_profile
		.video_quality
		.map_or_else(|| "-".to_owned(), |value| value.to_string());
	Some(format!("bitrate={bitrate}, quality={quality}"))
}

fn transcode_profile_summary(profile: &TranscodeProfile) -> String {
	let codec = profile
		.video_codec
		.map_or_else(|| "auto".to_owned(), |value| value.to_string());
	let default_bitrate = profile.default_video_bitrate.clone().unwrap_or_else(|| "-".to_owned());
	let default_quality = profile
		.default_video_quality
		.map_or_else(|| "-".to_owned(), |value| value.to_string());
	let mut summary = format!("codec={codec}, default_bitrate={default_bitrate}, default_quality={default_quality}");
	let codec_overrides = [
		("H264", &profile.h264),
		("H265", &profile.h265),
		("AV1", &profile.av1),
		("VP8", &profile.vp8),
		("VP9", &profile.vp9),
	]
	.iter()
	.filter_map(|(codec_name, codec_profile)| {
		codec_profile_summary(codec_profile)
			.map(|codec_profile_summary| format!("{codec_name}({codec_profile_summary})"))
	})
	.collect::<Vec<_>>();
	if !codec_overrides.is_empty() {
		summary.push_str(", overrides=");
		summary.push_str(codec_overrides.join("; ").as_str());
	}
	summary
}

fn user_transcode_profiles() -> Result<BTreeMap<String, TranscodeProfile>, TranscodeVideoProfileError> {
	let home_dir = home::home_dir().ok_or(TranscodeVideoProfileError::UnableToLocateHomeDirectory)?;
	let config_path = home_dir.join(USER_CONFIG_FILE_HOME_RELATIVE_PATH);
	if !config_path.exists() {
		return Ok(BTreeMap::new());
	}
	let config_contents =
		std::fs::read_to_string(&config_path).map_err(|error| TranscodeVideoProfileError::ConfigReadError {
			path: config_path.clone(),
			error,
		})?;
	let config_toml = toml::from_str::<HDVideoToolConfigToml>(&config_contents).map_err(|error| {
		TranscodeVideoProfileError::ConfigParseError {
			path: config_path.clone(),
			error,
		}
	})?;
	config_toml
		.transcode_video
		.profiles
		.into_iter()
		.map(|(profile_name, profile_toml)| {
			let profile = TranscodeProfile::from_toml(&profile_name, profile_toml)?;
			Ok((profile_name, profile))
		})
		.collect()
}

fn built_in_transcode_profiles() -> BTreeMap<String, TranscodeProfile> {
	let mut profiles = BTreeMap::new();
	profiles.insert(TRANSCODE_PROFILE_DIGITAL_FPV.to_owned(), TranscodeProfile::default());
	profiles.insert(
		TRANSCODE_PROFILE_ANALOG_FPV.to_owned(),
		TranscodeProfile {
			default_video_quality: Some(140),
			..TranscodeProfile::default()
		},
	);
	profiles
}

fn available_transcode_profiles() -> Result<BTreeMap<String, TranscodeProfile>, TranscodeVideoProfileError> {
	let mut profiles = built_in_transcode_profiles();
	profiles.extend(user_transcode_profiles()?);
	Ok(profiles)
}

/// Returns all available transcode profiles, including built-ins and user-defined profiles.
///
/// If a user-defined profile name matches a built-in profile name, the user-defined profile
/// overrides the built-in profile.
///
/// # Errors
/// Returns `TranscodeVideoProfileError` if user profile loading/parsing fails.
pub fn transcode_profiles_display() -> Result<String, TranscodeVideoProfileError> {
	let mut lines = vec!["Available profiles:".to_owned()];
	lines.extend(
		available_transcode_profiles()?
			.into_iter()
			.map(|(profile_name, profile)| format!("  {profile_name}: {}", transcode_profile_summary(&profile))),
	);
	lines.push(format!("Custom profiles file: ~/{USER_CONFIG_FILE_HOME_RELATIVE_PATH}"));
	Ok(lines.join("\n"))
}

#[derive(Debug, Error)]
pub enum OutputVideoFileError {
	#[error("input has no file name")]
	InputHasNoFileName,
	#[error("input has no extension")]
	InputHasNoExtension,
	#[error(transparent)]
	TranscodeVideoProfileError(#[from] TranscodeVideoProfileError),
}

impl TranscodeVideoArgs {
	fn profile(&self) -> Result<Option<TranscodeProfile>, TranscodeVideoProfileError> {
		let Some(profile_name) = self.profile.as_deref() else {
			return Ok(None);
		};

		available_transcode_profiles()?
			.get(profile_name)
			.cloned()
			.ok_or_else(|| TranscodeVideoProfileError::UnknownProfile {
				profile_name: profile_name.to_owned(),
			})
			.map(Some)
	}

	fn profile_requested_video_codec(&self) -> Result<Option<video::Codec>, TranscodeVideoProfileError> {
		Ok(self.profile()?.and_then(|profile| profile.video_codec))
	}

	/// Resolves the output video bitrate according to CLI values, profile values and defaults.
	///
	/// # Errors
	/// Returns `TranscodeVideoProfileError` if profile loading/parsing fails.
	pub fn resolved_video_bitrate(&self, video_codec: video::Codec) -> Result<String, TranscodeVideoProfileError> {
		if let Some(video_bitrate) = &self.video_bitrate {
			return Ok(video_bitrate.clone());
		}

		if let Some(profile) = self.profile()? {
			let codec_profile = profile.codec_profile(video_codec);
			if let Some(video_bitrate) = &codec_profile.video_bitrate {
				return Ok(video_bitrate.clone());
			}
			if let Some(default_video_bitrate) = &profile.default_video_bitrate {
				return Ok(default_video_bitrate.clone());
			}
		}

		Ok(DEFAULT_VIDEO_BITRATE.to_owned())
	}

	/// Resolves the output video quality according to CLI values, profile values and defaults.
	///
	/// # Errors
	/// Returns `TranscodeVideoProfileError` if profile loading/parsing fails.
	pub fn resolved_video_quality(&self, video_codec: video::Codec) -> Result<Option<u8>, TranscodeVideoProfileError> {
		if let Some(video_quality) = self.video_quality {
			return Ok(Some(video_quality));
		}

		if let Some(profile) = self.profile()? {
			let codec_profile = profile.codec_profile(video_codec);
			if let Some(video_quality) = codec_profile.video_quality {
				return Ok(Some(video_quality));
			}
			if let Some(default_video_quality) = profile.default_video_quality {
				return Ok(Some(default_video_quality));
			}
		}

		Ok(None)
	}

	#[must_use]
	pub fn video_audio_fix(&self) -> Option<video::AudioFixType> {
		use video::AudioFixType as AFT;
		match (self.fix_audio, self.fix_audio_sync, self.fix_audio_volume) {
			(true, _, _) | (false, true, true) => Some(AFT::SyncAndVolume),
			(false, true, false) => Some(AFT::Sync),
			(false, false, true) => Some(AFT::Volume),
			(false, false, false) => None,
		}
	}

	#[must_use]
	pub fn output_video_file_provided(&self) -> bool {
		self.output_video_file.is_some()
	}

	/// Returns the output video file path.
	///
	/// # Errors
	/// - Returns `OutputVideoFileError::InputHasNoFileName` if the input video file has no file name.
	/// - Returns `OutputVideoFileError::InputHasNoExtension` if the input video file has no extension.
	pub fn output_video_file(&self, with_osd: bool) -> Result<PathBuf, OutputVideoFileError> {
		Ok(if let Some(output_video_file) = &self.output_video_file {
			output_video_file.clone()
		} else {
			let mut output_file_stem = Path::new(
				self.input_video_file
					.file_stem()
					.ok_or(OutputVideoFileError::InputHasNoFileName)?,
			)
			.as_os_str()
			.to_os_string();
			let suffix = if with_osd { "_with_osd" } else { "_transcoded" };
			output_file_stem.push(suffix);
			let input_file_extension = self
				.input_video_file
				.extension()
				.ok_or(OutputVideoFileError::InputHasNoExtension)?;
			let (video_codec, hw_acceleration) = self.video_codec()?;
			let output_extension = if hw_acceleration.is_yes() {
				match video_codec {
					video::Codec::AV1 | video::Codec::H264 | video::Codec::H265 => OsString::from("mp4"),
					video::Codec::VP8 | video::Codec::VP9 => OsString::from("webm"),
				}
			} else {
				input_file_extension.to_os_string()
			};
			self.input_video_file
				.with_file_name(output_file_stem)
				.with_extension(output_extension)
		})
	}

	#[cfg(not(feature = "hwaccel"))]
	/// Resolves the output codec and hardware acceleration mode.
	///
	/// # Errors
	/// Returns `TranscodeVideoProfileError` if profile loading/parsing fails.
	pub fn video_codec(&self) -> Result<(video::Codec, HwAcceleratedEncoding), TranscodeVideoProfileError> {
		Ok((
			self.video_codec
				.or(self.profile_requested_video_codec()?)
				.unwrap_or(video::Codec::H265),
			HwAcceleratedEncoding::No,
		))
	}

	#[cfg(feature = "hwaccel")]
	/// Resolves the output codec and hardware acceleration mode.
	///
	/// # Errors
	/// Returns `TranscodeVideoProfileError` if profile loading/parsing fails.
	pub fn video_codec(&self) -> Result<(video::Codec, HwAcceleratedEncoding), TranscodeVideoProfileError> {
		const FALLBACK: (video::Codec, HwAcceleratedEncoding) = (video::Codec::H265, HwAcceleratedEncoding::No);
		let selected_video_codec = self.video_codec.or(self.profile_requested_video_codec()?);
		Ok(match selected_video_codec {
			None if self.no_hwaccel => FALLBACK,
			Some(video_codec) if self.no_hwaccel => (video_codec, HwAcceleratedEncoding::No),
			Some(video_codec) => match video::hw_accel::vaapi_cap_finder() {
				Some(hw_accel_cap) => (
					video_codec,
					HwAcceleratedEncoding::from(hw_accel_cap.can_encode(video_codec)),
				),
				None => (video_codec, HwAcceleratedEncoding::No),
			},
			None => {
				let hw_accel_codec = video::hw_accel::vaapi_cap_finder().and_then(|hw_accel_cap| {
					[video::Codec::AV1, video::Codec::H265]
						.iter()
						.find(|&video_codec| hw_accel_cap.can_encode(video_codec))
				});
				if let Some(hw_accel_codec) = hw_accel_codec {
					(*hw_accel_codec, HwAcceleratedEncoding::Yes)
				} else {
					FALLBACK
				}
			},
		})
	}
}
