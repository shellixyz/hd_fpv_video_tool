
use std::collections::BTreeSet;

use derive_more::Deref;
use getset::CopyGetters;
use rayon::{prelude::{IntoParallelRefIterator}, iter::plumbing::bridge as rayon_iter_bridge};

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

#[derive(Deref, Clone, CopyGetters)]
pub struct SortedFramesSlice<'a> {

    #[getset(get_copy = "pub")]
    kind: Kind,

    #[getset(get_copy = "pub")]
    font_variant: FontVariant,

    #[deref]
    frames: &'a [Frame]
}

pub trait AsSortedFramesSlice {
    fn as_sorted_frames_slice(&self) -> SortedFramesSlice;
}

impl AsSortedFramesSlice for SortedFrames {
    fn as_sorted_frames_slice(&self) -> SortedFramesSlice {
        SortedFramesSlice {
            kind: self.kind(),
            font_variant: self.font_variant(),
            frames: self.frames.as_slice(),
        }
    }
}

impl<'a> AsSortedFramesSlice for SortedFramesSlice<'a> {
    fn as_sorted_frames_slice(&self) -> SortedFramesSlice {
        // SortedFramesSlice {
        //     kind: self.kind(),
        //     font_variant: self.font_variant(),
        //     frames: self.frames,
        // }
        self.clone()
    }
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

    /// returns the video frame shifted index of the first frame which has a video frame shifted index greater than the specified first video frame
    pub fn first_video_frame_index(&self, first_video_frame: u32, video_frame_shift: i32) -> Option<u32> {
        let first_video_frame_index = first_video_frame as i32 - video_frame_shift;
        let first_frame_index = self.iter().position(|frame| (frame.index() as i32) >= first_video_frame_index)?;
        Some(u32::try_from(self[first_frame_index].index() as i32 + video_frame_shift).unwrap())
    }

    fn select_slice(&self, first_video_frame: u32, last_video_frame: Option<u32>, video_frame_shift: i32) -> &[Frame] {
        let first_video_frame_index = first_video_frame as i32 - video_frame_shift;
        let first_frame_index = self.iter().position(|frame| (frame.index() as i32) >= first_video_frame_index);

        match (first_frame_index, last_video_frame) {

            (Some(first_frame_index), Some(last_video_frame)) => {
                let last_video_frame_index = last_video_frame as i32 - video_frame_shift;
                let last_frame_index = self.iter().rposition(|frame| (frame.index() as i32) <= last_video_frame_index);
                last_frame_index.map(|index| &self[first_frame_index..=index]).unwrap_or(&[])
            },

            (Some(first_frame_index), None) => &self[first_frame_index..],

            (None, _) => &[],

        }
    }

    pub fn video_frame_indices(&self, first_video_frame: u32, last_video_frame: Option<u32>, video_frame_shift: i32) -> BTreeSet<u32> {
        self.select_slice(first_video_frame, last_video_frame, video_frame_shift)
            .iter().map(|frame| (frame.index() as i32 + video_frame_shift) as u32).collect()
    }

    /// returns an iterator which for each frame in the specified video frame interval returns the video frame shifted index and the frame
    pub fn shift_iter(&self, first_video_frame: u32, last_video_frame: Option<u32>, video_frame_shift: i32) -> ShiftIter {
        // let first_video_frame_index = first_video_frame as i32 - video_frame_shift;
        // let first_frame_index = self.iter().position(|frame| (frame.index() as i32) >= first_video_frame_index);

        // let osd_file_frames = match (first_frame_index, last_video_frame) {

        //     (Some(first_frame_index), Some(last_video_frame)) => {
        //         let last_video_frame_index = last_video_frame as i32 - video_frame_shift;
        //         let last_frame_index = self.iter().rposition(|frame| (frame.index() as i32) <= last_video_frame_index);
        //         last_frame_index.map(|index| &self[first_frame_index..=index]).unwrap_or(&[])
        //     },

        //     (Some(first_frame_index), None) => &self[first_frame_index..],

        //     (None, _) => &[],

        // };

        // ShiftIter {
        //     frames: self.select(first_video_frame, last_video_frame, video_frame_shift),
        //     video_frame_shift,
        // }
        let frames = self.select_slice(first_video_frame, last_video_frame, video_frame_shift);
        ShiftIter::new(frames, video_frame_shift)
    }

    // pub fn par_shift_iter(&self, first_video_frame: u32, last_video_frame: Option<u32>, video_frame_shift: i32) -> rayon::slice::Iter<(u32, &Frame)>  {
    //     // ParallelShiftIter {
    //     //     frames_iter: self.select(first_video_frame, last_video_frame, video_frame_shift).iter(),
    //     //     video_frame_shift,
    //     // }
    //         self.select(first_video_frame, last_video_frame, video_frame_shift).iter().map(|frame| {
    //             let actual_frame_video_frame_index = u32::try_from(frame.index() as i32 + video_frame_shift).unwrap();
    //             (actual_frame_video_frame_index, frame)
    //         }).collect::<Vec<_>>().par_iter()
    // }

    pub fn par_shift_iter(&self, first_video_frame: u32, last_video_frame: Option<u32>, video_frame_shift: i32) -> ParallelShiftIter {
        ParallelShiftIter {
            frames: self.select_slice(first_video_frame, last_video_frame, video_frame_shift),
            video_frame_shift,
        }
    }

    // pub fn shift_into_iter(&self, first_video_frame: u32, last_video_frame: Option<u32>, video_frame_shift: i32) -> ShiftIntoIter {
    //     ShiftIntoIter {
    //         frames_iter: self.select(first_video_frame, last_video_frame, video_frame_shift).to_vec().into_iter(),
    //         video_frame_shift,
    //     }
    // }

    pub fn video_frames_iter(&self, first_frame: u32, last_frame: Option<u32>, frame_shift: i32) -> VideoFramesIter {
        let first_video_frame_index = first_frame as i32 - frame_shift;
        let first_frame_index = self.iter().position(|frame| (frame.index() as i32) >= first_video_frame_index);
        let osd_file_frames = first_frame_index.map(|index| &self[index..]).unwrap_or(&[]);
        // let osd_file_frames = match first_frame_index {
        //     Some(index) => &self[index..],
        //     None => &[],
        // };

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

pub struct ShiftIter<'a> {
    frames: &'a [Frame],
    next_frame_index: usize,
    next_back_frame_index: usize,
    video_frame_shift: i32,
}

impl<'a> ShiftIter<'a> {
    fn new(frames: &'a [Frame], video_frame_shift: i32) -> Self {
        Self {
            frames,
            next_frame_index: 0,
            next_back_frame_index: frames.len() - 1,
            video_frame_shift,
        }
    }
}

impl<'a> Iterator for ShiftIter<'a> {
    type Item = (u32, &'a Frame);

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_frame_index == self.next_back_frame_index { return None }
        let frame = &self.frames[self.next_frame_index];
        self.next_frame_index += 1;
        let actual_frame_video_frame_index = u32::try_from(frame.index() as i32 + self.video_frame_shift).unwrap();
        Some((actual_frame_video_frame_index, frame))
    }

}

impl<'a> ExactSizeIterator for ShiftIter<'a> {
    fn len(&self) -> usize {
        self.frames.len()
    }
}

impl<'a> DoubleEndedIterator for ShiftIter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.next_frame_index == self.next_back_frame_index { return None }
        let frame = &self.frames[self.next_back_frame_index];
        self.next_back_frame_index -= 1;
        let actual_frame_video_frame_index = u32::try_from(frame.index() as i32 + self.video_frame_shift).unwrap();
        Some((actual_frame_video_frame_index, frame))
    }
}

pub struct ShiftIntoIter {
    frames_iter: std::vec::IntoIter<Frame>,
    video_frame_shift: i32,
}

impl Iterator for ShiftIntoIter {
    type Item = (u32, Frame);

    fn next(&mut self) -> Option<Self::Item> {
        let frame = self.frames_iter.next()?;
        let actual_frame_video_frame_index = u32::try_from(frame.index() as i32 + self.video_frame_shift).unwrap();
        Some((actual_frame_video_frame_index, frame))
    }

}

impl ExactSizeIterator for ShiftIntoIter {
    fn len(&self) -> usize {
        self.frames_iter.len()
    }
}

type ShiftIterItem<'a> = (u32, &'a Frame);

pub struct ParallelShiftIter<'a> {
    frames: &'a [Frame],
    video_frame_shift: i32,
}

impl<'a> rayon::iter::ParallelIterator for ParallelShiftIter<'a> {
    type Item = (u32, &'a Frame);

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: rayon::iter::plumbing::UnindexedConsumer<Self::Item> {
            // self.frames_iter.par_bridge().map(|frame| {
            //     let actual_frame_video_frame_index = u32::try_from(frame.index() as i32 + self.video_frame_shift).unwrap();
            //     (actual_frame_video_frame_index, frame)
            // }).drive_unindexed(consumer)
            rayon_iter_bridge(self, consumer)
    }
}

impl<'a> rayon::iter::IndexedParallelIterator for ParallelShiftIter<'a> {
    fn len(&self) -> usize {
        self.frames.len()
    }

    fn drive<C: rayon::iter::plumbing::Consumer<Self::Item>>(self, consumer: C) -> C::Result {
        rayon_iter_bridge(self, consumer)
    }

    fn with_producer<CB: rayon::iter::plumbing::ProducerCallback<Self::Item>>(self, callback: CB) -> CB::Output {
        callback.callback(self)
    }
}

impl<'a> rayon::iter::plumbing::Producer for ParallelShiftIter<'a> {
    type Item = ShiftIterItem<'a>;

    type IntoIter = ShiftIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        ShiftIter {
            frames: self.frames,
            video_frame_shift: self.video_frame_shift,
            next_frame_index: 0,
            next_back_frame_index: self.frames.len() - 1,
        }
    }

    fn split_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.frames.split_at(index);
        (
            ParallelShiftIter {
                frames: left,
                video_frame_shift: self.video_frame_shift,
            },
            ParallelShiftIter {
                frames: right,
                video_frame_shift: self.video_frame_shift,
            },
        )
    }
}