pub mod coordinates;
pub mod dji;
pub mod file;
pub mod font_dir;
pub mod font_variant;
pub mod item;
pub mod kind;
pub mod overlay;
pub mod region;
pub mod tile;
pub mod tile_indices;
pub mod tile_resize;
pub mod wsa;

use hd_fpv_osd_font_tool::dimensions::Dimensions as GenericDimensions;

pub type Dimensions = GenericDimensions<u32>;

pub use coordinates::{Coordinate, Coordinates, SignedCoordinate, SignedCoordinates, SignedRange as CoordinatesRange};
pub use font_dir::FontDir;
pub use font_variant::FontVariant;
pub use kind::Kind;
pub use region::Region;
pub use tile_indices::{TileIndex, TileIndices};
