
pub mod file;
pub mod font_variant;
pub mod font_dir;
pub mod kind;
pub mod overlay;
pub mod dji;
pub mod tile_resize;
pub mod tile;
pub mod region;
pub mod coordinates;
pub mod item;
pub mod tile_indices;

use hd_fpv_osd_font_tool::dimensions::Dimensions as GenericDimensions;

pub type Dimensions = GenericDimensions<u32>;

pub use region::Region as Region;
pub use coordinates::{
    Coordinate,
    Coordinates,
    SignedCoordinate,
    SignedCoordinates,
    SignedRange as CoordinatesRange};
pub use font_variant::FontVariant;
pub use kind::Kind;
pub use tile_indices::TileIndices;
pub use font_dir::FontDir;
