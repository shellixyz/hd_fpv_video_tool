use std::str::FromStr;

use derive_more::From;
use getset::Getters;
use thiserror::Error;

use crate::{osd, prelude::*};

#[derive(Debug, Clone, Getters)]
#[getset(get = "pub")]
pub struct Region {
	top_left_corner: osd::SignedCoordinates,
	dimensions: osd::Dimensions,
}

impl Region {
	#[must_use]
	pub fn new(top_left_corner: osd::SignedCoordinates, dimensions: osd::Dimensions) -> Self {
		Self {
			top_left_corner,
			dimensions,
		}
	}

	#[must_use]
	pub fn bottom_right_corner(&self) -> osd::SignedCoordinates {
		#[allow(clippy::cast_possible_truncation)]
		let width = self.dimensions.width as osd::SignedCoordinate;
		#[allow(clippy::cast_possible_truncation)]
		let height = self.dimensions.height as osd::SignedCoordinate;
		osd::SignedCoordinates {
			x: self.top_left_corner.x() + width - 1,
			y: self.top_left_corner.y() + height - 1,
		}
	}

	#[must_use]
	pub fn to_coordinates_range(&self) -> osd::CoordinatesRange {
		osd::CoordinatesRange::from(self)
	}
}

#[derive(Debug, Error)]
#[error("invalid OSD region format: {value}: {error}")]
pub enum FormatError {
	Origin {
		value: String,
		error: OSDCoordinatesFormatError,
	},
	Dimensions {
		value: String,
		error: GenericDimensionsFormatError,
	},
}

#[derive(Debug, Error, From)]
pub enum InvalidRegionString {
	#[error(transparent)]
	FormatError(FormatError),
	#[error("invalid dimensions: {0}: dimension component cannot be 0")]
	InvalidDimensionValue(String),
}

impl FromStr for Region {
	type Err = InvalidRegionString;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(if let Some((origin_s, dimensions_s)) = s.split_once(':') {
			let origin = osd::Coordinates::from_str(origin_s).map_err(|error| FormatError::Origin {
				value: origin_s.to_owned(),
				error,
			})?;
			let dimensions = osd::Dimensions::from_str(dimensions_s).map_err(|error| FormatError::Dimensions {
				value: dimensions_s.to_owned(),
				error,
			})?;
			if dimensions.width == 0 || dimensions.height == 0 {
				return Err(InvalidRegionString::InvalidDimensionValue(dimensions_s.to_owned()));
			}
			Region {
				top_left_corner: osd::SignedCoordinates::from(origin),
				dimensions,
			}
		} else {
			let origin = osd::Coordinates::from_str(s).map_err(|error| FormatError::Origin {
				value: s.to_owned(),
				error,
			})?;
			Region {
				top_left_corner: osd::SignedCoordinates::from(origin),
				dimensions: osd::Dimensions::new(1, 1),
			}
		})
	}
}
