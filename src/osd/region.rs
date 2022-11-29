
use std::str::FromStr;

use derive_more::From;
use getset::Getters;
use crate::prelude::*;
use thiserror::Error;

use super::coordinates::Range as CoordinatesRange;

#[derive(Debug, Clone, Getters)]
#[getset(get = "pub")]
pub struct Region {
    top_left_corner: OSDCoordinates,
    dimensions: OSDDimensions,
}

impl Region {

    pub fn bottom_right_corner(&self) -> OSDCoordinates {
        OSDCoordinates {
            x: self.top_left_corner.x() + self.dimensions.width - 1,
            y: self.top_left_corner.y() + self.dimensions.height - 1,
        }
    }

    pub fn to_coordinates_range(&self) -> CoordinatesRange {
        CoordinatesRange::from(self)
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
        error: GenericDimensionsFormatError
    }
}

#[derive(Debug, Error, From)]
pub enum InvalidRegionString {
    #[error(transparent)]
    FormatError(FormatError),
    #[error("invalid dimensions: {0}: dimension component cannot be 0")]
    InvalidDimensionValue(String)
}

impl FromStr for Region {
    type Err = InvalidRegionString;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.split_once(':') {

            Some((origin_s, dimensions_s)) => {
                let origin = OSDCoordinates::from_str(origin_s)
                    .map_err(|error| FormatError::Origin { value: origin_s.to_owned(), error })?;
                let dimensions = OSDDimensions::from_str(dimensions_s)
                    .map_err(|error| FormatError::Dimensions { value: dimensions_s.to_owned(), error })?;
                if dimensions.width == 0 || dimensions.height == 0 {
                    return Err(InvalidRegionString::InvalidDimensionValue(dimensions_s.to_owned()));
                }
                Region {
                    top_left_corner: origin,
                    dimensions
                }
            },

            None => {
                let origin = OSDCoordinates::from_str(s)
                    .map_err(|error| FormatError::Origin { value: s.to_owned(), error })?;
                Region {
                    top_left_corner: origin,
                    dimensions: OSDDimensions::new(1, 1),
                }
            },

        })
    }
}
