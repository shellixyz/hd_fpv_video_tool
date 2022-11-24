use byte_struct::*;
use derive_more::Deref;
use getset::{CopyGetters, Getters};

use super::tile_indices::{
    TileIndices,
    TileIndicesEnumeratorIter,
};

use crate::video::FrameIndex as VideoFrameIndex;


#[derive(ByteStruct, Debug, CopyGetters)]
#[getset(get_copy = "pub")]
#[byte_struct_le]
pub struct Header {
    frame_index: VideoFrameIndex,
    data_len: u32
}

#[derive(Debug, CopyGetters, Getters, Deref, Clone, PartialEq, Eq)]
pub struct Frame {
    #[getset(get_copy = "pub")]
    index: u32,

    #[getset(get = "pub")]
    #[deref] tile_indices: TileIndices
}

impl Frame {

    pub fn new(index: VideoFrameIndex, tile_indices: TileIndices) -> Self {
        Self { index, tile_indices }
    }

    pub fn enumerate_tile_indices(&self) -> TileIndicesEnumeratorIter {
        self.tile_indices().enumerate()
    }

}