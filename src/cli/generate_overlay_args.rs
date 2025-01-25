use std::{ffi::OsStr, path::PathBuf};

use anyhow::anyhow;
use clap::{Args, builder::StyledStr};
use derive_builder::Builder;
use getset::{CopyGetters, Getters};
use itertools::Itertools;
use strum::IntoEnumIterator;

use crate::{
	osd::{font_variant::FontVariant, item::LocationData},
	prelude::ScalingArgs,
	video,
};

use super::{font_options::FontOptions, start_end_args::StartEndArgs};
use crate::osd;

#[derive(Args, Getters, CopyGetters, Builder, Clone)]
#[getset(get = "pub")]
pub struct GenerateOverlayArgs {
	/// use the resolution from the specified video file to decide what kind of tiles (SD/HD) would best fit and also whether scaling should be used when in auto scaling mode
	#[clap(short = 'v', long, group("target_resolution_group"), value_parser)]
	#[getset(skip)]
	#[getset(get = "pub")]
	target_video_file: Option<PathBuf>,

	/// hide rectangular regions from the OSD
	///
	/// The parameter is a `;` separated list of regions.{n}
	/// The format for a region is: <left_x>,<top_y>[:<width>x<height>]{n}
	/// If the size is not specified it will default to 1x1
	#[clap(long, value_parser, value_delimiter = ';', value_name = "REGIONS")]
	hide_regions: Vec<osd::Region>,

	/// hide items from the OSD
	#[clap(long, value_parser, value_delimiter = ',', value_name = "ITEM_NAMES", help = osd_hide_items_arg_help())]
	hide_items: Vec<String>,

	#[clap(flatten)]
	start_end: StartEndArgs,

	#[clap(flatten)]
	scaling_args: ScalingArgs,

	#[clap(flatten)]
	font_options: FontOptions,

	/// Shift the output by that number of frames. Use this option to sync the OSD to a particular video.
	#[clap(short = 'o', long, value_parser, value_name = "frames", allow_negative_numbers(true))]
	#[getset(skip)]
	frame_shift: Option<i32>,

	/// path to FPV.WTF .osd file
	osd_file: PathBuf,
}

pub(crate) fn osd_hide_items_arg_help() -> StyledStr {
	let mut help = indoc::indoc! {"
        hide items from the OSD

        Available items (font variant: name list):
    "}
	.to_string();
	let font_variant_items = FontVariant::iter()
		.filter_map(|font_variant| {
			if font_variant.osd_items_location_data().is_empty() {
				None
			} else {
				let item_names_list = font_variant
					.osd_items_location_data()
					.iter()
					.map(LocationData::name)
					.join(", ");
				Some(format!("  - {font_variant}: {item_names_list}"))
			}
		})
		.join("\n");
	help.push_str(&font_variant_items);
	help.into()
}

impl GenerateOverlayArgs {
	pub fn check_valid(&self) -> anyhow::Result<()> {
		self.start_end().check_valid()?;
		if self.osd_file.extension().map(ToOwned::to_owned).unwrap_or_default() != OsStr::new("osd") {
			return Err(anyhow!("FPV.WTF OSD files should have the .osd extension"));
		}
		Ok(())
	}

	pub fn frame_shift(&self) -> anyhow::Result<i32> {
		Ok(match (self.frame_shift, &self.target_video_file) {
			(Some(frame_shift), _) => frame_shift,
			(None, Some(target_video_file)) => {
				if video::probe(target_video_file)?.has_audio() {
					let frame_shift = crate::osd::dji::AU_OSD_FRAME_SHIFT;
					log::info!(
						"target video file contains audio, assuming DJI AU origin, applying {frame_shift} OSD frames shift"
					);
					frame_shift
				} else {
					0
				}
			},
			(None, None) => 0,
		})
	}
}
