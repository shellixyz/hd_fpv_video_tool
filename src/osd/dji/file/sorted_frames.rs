
use derive_more::Deref;
use getset::CopyGetters;

use super::{frame::Frame, tile_indices::TileIndex, FontVariant};

use crate::{video::FrameIndex as VideoFrameIndex, osd::dji::Kind};


#[derive(Deref, Clone, CopyGetters)]
pub struct SortedFrames {

    #[getset(get_copy = "pub")]
    kind: Kind,

    #[getset(get_copy = "pub")]
    font_variant: FontVariant,

    #[deref]
    frames: Vec<Frame>
}

impl SortedFrames {

    pub fn new(kind: Kind, font_variant: FontVariant, frames: Vec<Frame>) -> Self {
        Self { frames, kind, font_variant }
    }

    pub fn highest_video_frame_index(&self) -> Option<VideoFrameIndex> {
        self.last().map(Frame::index)
    }

    pub fn highest_used_tile_index(&self) -> Option<TileIndex> {
        self.iter().flat_map(|frame| frame.tile_indices().as_slice()).max().cloned()
    }

    pub fn video_frames_iter(&self, first_frame: u32, last_frame: Option<u32>, frame_shift: i32) -> VideoFramesIter {
        let first_video_frame_index = first_frame as i32 - frame_shift;
        let first_frame_index = self.iter().position(|frame| (frame.index() as i32) >= first_video_frame_index);
        let osd_file_frames = match first_frame_index {
            Some(index) => &self[index..],
            None => &[],
        };

        VideoFramesIter {
            frames: osd_file_frames,
            frame_index: 0,
            video_frame_index: first_frame,
            last_video_frame_index: last_frame,
            video_frame_shift: frame_shift,
        }
    }

}

pub struct VideoFramesIter<'a> {
    frames: &'a [Frame],
    frame_index: usize,
    video_frame_index: u32,
    last_video_frame_index: Option<u32>,
    video_frame_shift: i32,
}

impl<'a> Iterator for VideoFramesIter<'a> {
    type Item = Option<&'a Frame>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.last_video_frame_index {
            Some(last_frame) => {
                if self.video_frame_index > last_frame {
                    return None;
                } else if self.frame_index >= self.frames.len() {
                    self.video_frame_index += 1;
                    return Some(None);
                }
            },
            None => {
                if self.frame_index >= self.frames.len() {
                    return None;
                }
            }
        }

        let current_frame = &self.frames[self.frame_index];
        let actual_frame_video_frame_index = current_frame.index() as i32 + self.video_frame_shift;

        let frame =
            if (self.video_frame_index as i32) < actual_frame_video_frame_index {
                None
            } else {
                self.frame_index += 1;
                Some(current_frame)
            };

        self.video_frame_index += 1;

        Some(frame)
    }

}

impl<'a> ExactSizeIterator for VideoFramesIter<'a> {
    fn len(&self) -> usize {
        match self.last_video_frame_index {
            Some(last_video_frame_index) => last_video_frame_index as usize + 1,
            None => self.frames.last().map(|frame| frame.index() + 1).unwrap_or(0) as usize,
        }
    }
}