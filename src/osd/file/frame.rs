use derive_more::Deref;
use getset::{CopyGetters, Getters};

use crate::{
	osd::{
		FontVariant, Region, TileIndices,
		tile_indices::{TileIndicesEnumeratorIter, UnknownOSDItem},
	},
	video,
};

#[derive(Debug, CopyGetters, Getters, Deref, Clone, PartialEq, Eq)]
pub struct Frame {
	#[getset(get_copy = "pub")]
	index: u32,

	#[getset(get = "pub")]
	#[deref]
	tile_indices: TileIndices,
}

impl Frame {
	#[must_use]
	pub fn new(index: video::FrameIndex, tile_indices: TileIndices) -> Self {
		Self { index, tile_indices }
	}

	#[must_use]
	pub fn enumerate_tile_indices(&self) -> TileIndicesEnumeratorIter<'_> {
		self.tile_indices().enumerate()
	}

	#[must_use]
	pub fn with_erased_regions(&self, regions: &[Region]) -> Self {
		let mut tile_indices = self.tile_indices.clone();
		tile_indices.erase_regions(regions);
		Self::new(self.index, tile_indices)
	}

	/// Erases OSD items by their names for the given font variant.
	///
	/// # Errors
	/// - Returns `UnknownOSDItem` if any of the provided item names do not correspond to known OSD items for the specified font variant.
	pub fn with_erased_osd_items(
		&self,
		font_variant: FontVariant,
		item_names: &[String],
	) -> Result<Self, UnknownOSDItem> {
		let mut tile_indices = self.tile_indices.clone();
		tile_indices.erase_osd_items(font_variant, item_names)?;
		Ok(Self::new(self.index, tile_indices))
	}
}
