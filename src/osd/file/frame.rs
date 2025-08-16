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
	pub fn new(index: video::FrameIndex, tile_indices: TileIndices) -> Self {
		Self { index, tile_indices }
	}

	pub fn enumerate_tile_indices(&self) -> TileIndicesEnumeratorIter<'_> {
		self.tile_indices().enumerate()
	}

	pub fn with_erased_regions(&self, regions: &[Region]) -> Self {
		let mut tile_indices = self.tile_indices.clone();
		tile_indices.erase_regions(regions);
		Self::new(self.index, tile_indices)
	}

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
