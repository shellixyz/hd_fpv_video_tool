
use std::str::FromStr;

use derive_more::From;
use getset::Getters;
use crate::prelude::*;
use thiserror::Error;




#[derive(Debug, Clone, Getters)]
#[getset(get = "pub")]
pub struct Region {
    top_left_corner: super::SignedCoordinates,
    dimensions: super::Dimensions,
}

impl Region {

    pub fn new(top_left_corner: super::SignedCoordinates, dimensions: super::Dimensions) -> Self {
        Self { top_left_corner, dimensions }
    }

    pub fn new4(x: super::SignedCoordinate, y: super::SignedCoordinate, width: super::Dimension, height: super::Dimension) -> Self {
        Self {
            top_left_corner: super::SignedCoordinates::new(x, y),
            dimensions: super::Dimensions::new(width, height)
        }
    }

    pub fn bottom_right_corner(&self) -> super::SignedCoordinates {
        super::SignedCoordinates {
            x: self.top_left_corner.x() + self.dimensions.width as super::SignedCoordinate - 1,
            y: self.top_left_corner.y() + self.dimensions.height as super::SignedCoordinate - 1,
        }
    }

    pub fn to_coordinates_range(&self) -> super::coordinates::SignedRange {
        super::coordinates::SignedRange::from(self)
    }

}

#[derive(Debug, Error)]
#[error("invalid OSD region format: {value}: {error}")]
pub enum FormatError {
    Origin {
        value: String,
        error: super::CoordinatesFormatError,
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
                let origin = super::Coordinates::from_str(origin_s)
                    .map_err(|error| FormatError::Origin { value: origin_s.to_owned(), error })?;
                let dimensions = super::Dimensions::from_str(dimensions_s)
                    .map_err(|error| FormatError::Dimensions { value: dimensions_s.to_owned(), error })?;
                if dimensions.width == 0 || dimensions.height == 0 {
                    return Err(InvalidRegionString::InvalidDimensionValue(dimensions_s.to_owned()));
                }
                Region {
                    top_left_corner: super::SignedCoordinates::from(origin),
                    dimensions
                }
            },

            None => {
                let origin = super::Coordinates::from_str(s)
                    .map_err(|error| FormatError::Origin { value: s.to_owned(), error })?;
                Region {
                    top_left_corner: super::SignedCoordinates::from(origin),
                    dimensions: super::Dimensions::new(1, 1),
                }
            },

        })
    }
}
