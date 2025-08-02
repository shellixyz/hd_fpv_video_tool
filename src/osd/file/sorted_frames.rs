use derive_more::Deref;
use getset::CopyGetters;
use rayon::iter::plumbing::Consumer as RayonConsumer;
use rayon::iter::plumbing::Producer as RayonProducer;
use rayon::iter::plumbing::ProducerCallback as RayonProducerCallback;
use rayon::iter::plumbing::UnindexedConsumer as RayonUnindexedConsumer;
use rayon::iter::{IndexedParallelIterator, ParallelIterator, plumbing::bridge as rayon_iter_bridge};
use strum::EnumIter;

use super::Frame;

use crate::{
	osd::{FontVariant, Kind, tile_indices::TileIndex},
	video::FrameIndex as VideoFrameIndex,
};

#[derive(Deref, Clone, CopyGetters)]
pub struct SortedUniqFrames {
	#[getset(get_copy = "pub")]
	kind: Kind,

	#[getset(get_copy = "pub")]
	font_variant: FontVariant,

	#[deref]
	frames: Vec<Frame>,
}

impl SortedUniqFrames {
	pub fn new(kind: Kind, font_variant: FontVariant, frames: Vec<Frame>) -> Self {
		Self {
			frames,
			kind,
			font_variant,
		}
	}
}

#[derive(Deref, Clone, CopyGetters)]
pub struct SortedUniqFramesForVideoSlice<'a> {
	#[getset(get_copy = "pub")]
	kind: Kind,

	#[getset(get_copy = "pub")]
	font_variant: FontVariant,

	video_frame_shift: i32,

	#[getset(get_copy = "pub")]
	first_video_frame: u32,

	#[getset(get_copy = "pub")]
	last_video_frame: Option<u32>,

	#[deref]
	frames: &'a [Frame],
}

impl<'a> SortedUniqFramesForVideoSlice<'a> {
	pub fn new(
		kind: Kind,
		font_variant: FontVariant,
		frames: &'a [Frame],
		first_video_frame: u32,
		last_video_frame: Option<u32>,
		video_frame_shift: i32,
	) -> Self {
		Self {
			frames,
			kind,
			font_variant,
			first_video_frame,
			last_video_frame,
			video_frame_shift,
		}
	}

	pub fn video_frames_rel_index_iter(&self, eof_action: EndOfFramesAction) -> VideoFramesRelIndexIter {
		let index_shift = self.video_frame_shift - self.first_video_frame as i32;
		VideoFramesRelIndexIter::new(self.frames, index_shift, eof_action, self.last_video_frame)
	}

	pub fn video_frames_rel_index_par_iter(&self, eof_action: EndOfFramesAction) -> ParallelVideoFramesRelIndexIter {
		let index_shift = self.video_frame_shift - self.first_video_frame as i32;
		ParallelVideoFramesRelIndexIter::new(self.frames, index_shift, eof_action, self.last_video_frame)
	}
}

pub trait AsSortedFramesForVideoSlice {
	fn as_sorted_frames_slice(&self) -> SortedUniqFramesForVideoSlice;
}

impl AsSortedFramesForVideoSlice for SortedUniqFrames {
	fn as_sorted_frames_slice(&self) -> SortedUniqFramesForVideoSlice {
		SortedUniqFramesForVideoSlice {
			kind: self.kind(),
			font_variant: self.font_variant(),
			frames: self.frames.as_slice(),
			first_video_frame: 0,
			last_video_frame: None,
			video_frame_shift: 0,
		}
	}
}

impl AsSortedFramesForVideoSlice for SortedUniqFramesForVideoSlice<'_> {
	fn as_sorted_frames_slice(&self) -> SortedUniqFramesForVideoSlice {
		self.clone()
	}
}

pub trait GetFrames {
	fn frames(&self) -> &[Frame];
}

impl GetFrames for SortedUniqFrames {
	fn frames(&self) -> &[Frame] {
		self.frames.as_slice()
	}
}

impl GetFrames for SortedUniqFramesForVideoSlice<'_> {
	fn frames(&self) -> &[Frame] {
		self.frames
	}
}

#[derive(Deref)]
pub struct SortedUniqFrameIndices(Vec<VideoFrameIndex>);

pub trait GetFramesExt {
	fn highest_video_frame_index(&self) -> Option<VideoFrameIndex>;
	fn highest_used_tile_index(&self) -> Option<TileIndex>;
	fn first_video_frame_index(&self, first_video_frame: u32, video_frame_shift: i32) -> Option<u32>;
	fn video_frame_indices(&self, video_frame_shift: i32) -> SortedUniqFrameIndices;
	fn shift_iter(&self, video_frame_shift: i32) -> ShiftIter;
	fn par_shift_iter(&self, video_frame_shift: i32) -> ParallelShiftIter;
	fn video_frames_iter(&self, first_frame: u32, last_frame: Option<u32>, frame_shift: i32) -> VideoFramesIter;
}

impl<T> GetFramesExt for T
where
	T: GetFrames,
{
	fn highest_video_frame_index(&self) -> Option<VideoFrameIndex> {
		self.frames().last().map(Frame::index)
	}

	fn highest_used_tile_index(&self) -> Option<TileIndex> {
		self.frames()
			.iter()
			.flat_map(|frame| frame.tile_indices().as_slice())
			.max()
			.cloned()
	}

	/// returns the video frame shifted index of the first frame which has a video frame shifted index greater than the specified first video frame
	fn first_video_frame_index(&self, first_video_frame: u32, video_frame_shift: i32) -> Option<u32> {
		let first_video_frame_index = first_video_frame as i32 - video_frame_shift;
		let first_frame_index = self
			.frames()
			.iter()
			.position(|frame| (frame.index() as i32) >= first_video_frame_index)?;
		Some(u32::try_from(self.frames()[first_frame_index].index() as i32 + video_frame_shift).unwrap())
	}

	fn video_frame_indices(&self, video_frame_shift: i32) -> SortedUniqFrameIndices {
		SortedUniqFrameIndices(
			self.frames()
				.iter()
				.map(|frame| (frame.index() as i32 + video_frame_shift) as u32)
				.collect(),
		)
	}

	fn shift_iter(&self, video_frame_shift: i32) -> ShiftIter {
		ShiftIter::new(self.frames(), video_frame_shift)
	}

	fn par_shift_iter(&self, video_frame_shift: i32) -> ParallelShiftIter {
		ParallelShiftIter {
			frames: self.frames(),
			video_frame_shift,
		}
	}

	fn video_frames_iter(&self, first_frame: u32, last_frame: Option<u32>, frame_shift: i32) -> VideoFramesIter {
		let first_video_frame_index = first_frame as i32 - frame_shift;
		let first_frame_index = self
			.frames()
			.iter()
			.position(|frame| (frame.index() as i32) >= first_video_frame_index);
		let osd_file_frames = first_frame_index.map(|index| &self.frames()[index..]).unwrap_or(&[]);

		VideoFramesIter {
			frames: osd_file_frames,
			frame_index: 0,
			video_frame_index: first_frame,
			last_video_frame_index: last_frame,
			video_frame_shift: frame_shift,
		}
	}
}

impl SortedUniqFrames {
	pub fn select_slice(
		&self,
		first_video_frame: u32,
		last_video_frame: Option<u32>,
		video_frame_shift: i32,
	) -> SortedUniqFramesForVideoSlice {
		let first_video_frame_index = first_video_frame as i32 - video_frame_shift;
		let first_frame_index = self
			.frames()
			.iter()
			.position(|frame| (frame.index() as i32) >= first_video_frame_index);

		let frames = match (first_frame_index, last_video_frame) {
			(Some(first_frame_index), Some(last_video_frame)) => {
				let last_video_frame_index = last_video_frame as i32 - video_frame_shift;
				let last_frame_index = self
					.frames()
					.iter()
					.rposition(|frame| (frame.index() as i32) <= last_video_frame_index);
				last_frame_index
					.map(|index| &self.frames()[first_frame_index..=index])
					.unwrap_or(&[])
			},

			(Some(first_frame_index), None) => &self.frames()[first_frame_index..],

			(None, _) => &[],
		};

		SortedUniqFramesForVideoSlice::new(
			self.kind(),
			self.font_variant(),
			frames,
			first_video_frame,
			last_video_frame,
			video_frame_shift,
		)
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
			},
		}

		let current_frame = &self.frames[self.frame_index];
		let actual_frame_video_frame_index = current_frame.index() as i32 + self.video_frame_shift;

		let frame = if (self.video_frame_index as i32) < actual_frame_video_frame_index {
			None
		} else {
			self.frame_index += 1;
			Some(current_frame)
		};

		self.video_frame_index += 1;

		Some(frame)
	}
}

impl ExactSizeIterator for VideoFramesIter<'_> {
	fn len(&self) -> usize {
		match self.last_video_frame_index {
			Some(last_video_frame_index) => last_video_frame_index as usize + 1,
			None => self.frames.last().map(|frame| frame.index() + 1).unwrap_or(0) as usize,
		}
	}
}

pub struct ShiftIter<'a> {
	frames: &'a [Frame],
	frame_index: isize,
	back_frame_index: isize,
	video_frame_shift: i32,
}

impl<'a> ShiftIter<'a> {
	fn new(frames: &'a [Frame], video_frame_shift: i32) -> Self {
		Self {
			frames,
			frame_index: -1,
			back_frame_index: frames.len() as isize,
			video_frame_shift,
		}
	}
}

impl<'a> Iterator for ShiftIter<'a> {
	type Item = (u32, &'a Frame);

	fn next(&mut self) -> Option<Self::Item> {
		self.frame_index += 1;
		if self.frame_index == self.back_frame_index {
			return None;
		}
		let frame = &self.frames[self.frame_index as usize];
		let actual_frame_video_frame_index = u32::try_from(frame.index() as i32 + self.video_frame_shift).unwrap();
		Some((actual_frame_video_frame_index, frame))
	}
}

impl ExactSizeIterator for ShiftIter<'_> {
	fn len(&self) -> usize {
		self.frames.len()
	}
}

impl DoubleEndedIterator for ShiftIter<'_> {
	fn next_back(&mut self) -> Option<Self::Item> {
		self.back_frame_index -= 1;
		if self.frame_index == self.back_frame_index {
			return None;
		}
		let frame = &self.frames[self.back_frame_index as usize];
		let actual_frame_video_frame_index = u32::try_from(frame.index() as i32 + self.video_frame_shift).unwrap();
		Some((actual_frame_video_frame_index, frame))
	}
}

type ShiftIterItem<'a> = (u32, &'a Frame);

pub struct ParallelShiftIter<'a> {
	frames: &'a [Frame],
	video_frame_shift: i32,
}

impl<'a> ParallelIterator for ParallelShiftIter<'a> {
	type Item = (u32, &'a Frame);

	fn drive_unindexed<C>(self, consumer: C) -> C::Result
	where
		C: RayonUnindexedConsumer<Self::Item>,
	{
		rayon_iter_bridge(self, consumer)
	}
}

impl IndexedParallelIterator for ParallelShiftIter<'_> {
	fn len(&self) -> usize {
		self.frames.len()
	}

	fn drive<C: RayonConsumer<Self::Item>>(self, consumer: C) -> C::Result {
		rayon_iter_bridge(self, consumer)
	}

	fn with_producer<CB: RayonProducerCallback<Self::Item>>(self, callback: CB) -> CB::Output {
		callback.callback(self)
	}
}

impl<'a> RayonProducer for ParallelShiftIter<'a> {
	type Item = ShiftIterItem<'a>;

	type IntoIter = ShiftIter<'a>;

	fn into_iter(self) -> Self::IntoIter {
		ShiftIter::new(self.frames, self.video_frame_shift)
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VideoFramesRelIndexIterItem<'a> {
	Existing { rel_index: u32, frame: &'a Frame },
	FirstNonExisting,
	NonExisting { prev_rel_index: u32, rel_index: u32 },
}

#[derive(Debug, Clone, Copy, EnumIter, strum::Display)]
pub enum EndOfFramesAction {
	ContinueToLastVideoFrame,
	Stop,
}

#[derive(Debug)]
pub struct VideoFramesRelIndexIter<'a> {
	frames: &'a [Frame],
	frame_index: usize,
	prev_index: u32,
	video_frame_index: u32,
	index_shift: i32,
	// used when the iterator is split to know that this is the last iterator so the one responsible for the eof action
	end_iter: bool,
	eof_action: EndOfFramesAction,
	last_video_frame: Option<u32>,
}

impl ExactSizeIterator for VideoFramesRelIndexIter<'_> {
	fn len(&self) -> usize {
		use EndOfFramesAction::*;
		match (self.eof_action, self.last_video_frame) {
			(ContinueToLastVideoFrame, Some(last_video_frame)) => (last_video_frame + 1) as usize,
			(Stop, _) | (ContinueToLastVideoFrame, None) => self
				.frames
				.last()
				.map(|frame| u32::try_from(frame.index() as i32 + self.index_shift + 1).unwrap())
				.unwrap_or_default() as usize,
		}
	}
}

impl<'a> VideoFramesRelIndexIter<'a> {
	// NOTE: last_video_frame should NOT be shifted prior to calling this function
	pub fn new(
		frames: &'a [Frame],
		index_shift: i32,
		eof_action: EndOfFramesAction,
		last_video_frame: Option<u32>,
	) -> Self {
		Self {
			frames,
			frame_index: 0,
			video_frame_index: 0,
			prev_index: 0,
			index_shift,
			end_iter: true,
			eof_action,
			last_video_frame: last_video_frame
				.map(|index| u32::try_from(index as i32 + index_shift).unwrap_or_default()),
		}
	}
}

impl<'a> From<ParallelVideoFramesRelIndexIter<'a>> for VideoFramesRelIndexIter<'a> {
	fn from(pvfrii: ParallelVideoFramesRelIndexIter<'a>) -> Self {
		Self {
			frames: pvfrii.frames,
			frame_index: pvfrii.frame_index,
			prev_index: pvfrii.prev_index,
			video_frame_index: pvfrii.video_frame_index,
			index_shift: pvfrii.index_shift,
			end_iter: pvfrii.end_iter,
			eof_action: pvfrii.eof_action,
			last_video_frame: pvfrii.last_video_frame,
		}
	}
}

impl<'a> Iterator for VideoFramesRelIndexIter<'a> {
	type Item = VideoFramesRelIndexIterItem<'a>;

	fn next(&mut self) -> Option<Self::Item> {
		if self.frame_index >= self.frames.len() {
			if self.end_iter
				&& matches!(self.eof_action, EndOfFramesAction::ContinueToLastVideoFrame)
				&& matches!(self.last_video_frame, Some(lvf) if self.video_frame_index <= lvf )
			{
				let item = VideoFramesRelIndexIterItem::NonExisting {
					prev_rel_index: self.prev_index,
					rel_index: self.video_frame_index,
				};
				self.video_frame_index += 1;
				return Some(item);
			}
			return None;
		}

		let frame = &self.frames[self.frame_index];
		let actual_frame_vfi = u32::try_from(frame.index() as i32 + self.index_shift).unwrap();

		let item = match self.video_frame_index {
			0 if actual_frame_vfi > 0 => {
				self.prev_index = 0;
				VideoFramesRelIndexIterItem::FirstNonExisting
			},
			vfi if vfi < actual_frame_vfi => VideoFramesRelIndexIterItem::NonExisting {
				prev_rel_index: self.prev_index,
				rel_index: self.video_frame_index,
			},
			vfi if vfi == actual_frame_vfi => {
				self.frame_index += 1;
				self.prev_index = self.video_frame_index;
				VideoFramesRelIndexIterItem::Existing {
					rel_index: self.video_frame_index,
					frame,
				}
			},
			_ => {
				// if that block is reached it means the frames we are iterating over were either not sorted by index
				// or each frame did not have an uniq index. Should not be possible if the iterator was created
				// from SortedUniqFrames or SortedUniqFramesForVideoSlice  iter methods
				unreachable!()
			},
		};

		self.video_frame_index += 1;

		Some(item)
	}
}

impl DoubleEndedIterator for VideoFramesRelIndexIter<'_> {
	fn next_back(&mut self) -> Option<Self::Item> {
		unimplemented!()
	}
}

pub struct ParallelVideoFramesRelIndexIter<'a> {
	frames: &'a [Frame],
	frame_index: usize,
	prev_index: u32,
	video_frame_index: u32,
	index_shift: i32,
	// used when the iterator is split to know that this is the last iterator so the one responsible for the eof action
	end_iter: bool,
	eof_action: EndOfFramesAction,
	last_video_frame: Option<u32>,
}

impl<'a> ParallelVideoFramesRelIndexIter<'a> {
	pub fn new(
		frames: &'a [Frame],
		index_shift: i32,
		eof_action: EndOfFramesAction,
		last_video_frame: Option<u32>,
	) -> Self {
		Self {
			frames,
			frame_index: 0,
			video_frame_index: 0,
			prev_index: 0,
			index_shift,
			end_iter: true,
			eof_action,
			last_video_frame: last_video_frame
				.map(|index| u32::try_from(index as i32 + index_shift).unwrap_or_default()),
		}
	}
}

impl<'a> ParallelIterator for ParallelVideoFramesRelIndexIter<'a> {
	type Item = VideoFramesRelIndexIterItem<'a>;

	fn drive_unindexed<C>(self, consumer: C) -> C::Result
	where
		C: RayonUnindexedConsumer<Self::Item>,
	{
		rayon_iter_bridge(self, consumer)
	}
}

impl IndexedParallelIterator for ParallelVideoFramesRelIndexIter<'_> {
	fn len(&self) -> usize {
		use EndOfFramesAction::*;
		match (self.eof_action, self.last_video_frame) {
			(ContinueToLastVideoFrame, Some(last_video_frame)) => (last_video_frame + 1) as usize,
			(Stop, _) | (ContinueToLastVideoFrame, None) => self
				.frames
				.last()
				.map(|frame| u32::try_from(frame.index() as i32 + self.index_shift + 1).unwrap())
				.unwrap_or_default() as usize,
		}
	}

	fn drive<C: RayonConsumer<Self::Item>>(self, consumer: C) -> C::Result {
		rayon_iter_bridge(self, consumer)
	}

	fn with_producer<CB: RayonProducerCallback<Self::Item>>(self, callback: CB) -> CB::Output {
		callback.callback(self)
	}
}

impl<'a> RayonProducer for ParallelVideoFramesRelIndexIter<'a> {
	type Item = VideoFramesRelIndexIterItem<'a>;

	type IntoIter = VideoFramesRelIndexIter<'a>;

	fn into_iter(self) -> Self::IntoIter {
		self.into()
	}

	fn split_at(self, _index: usize) -> (Self, Self) {
		let index = self.frames.len() / 2;
		let (left, right) = if index > self.frame_index {
			let (left_frames, right_frames) = self.frames.split_at(index);
			let left = Self {
				frames: left_frames,
				frame_index: self.frame_index,
				prev_index: self.prev_index,
				video_frame_index: self.video_frame_index,
				index_shift: self.index_shift,
				end_iter: false,
				eof_action: self.eof_action,
				last_video_frame: self.last_video_frame,
			};
			let left_last_frame_index = left_frames.last().map(|frame| frame.index()).unwrap_or_default();
			let right = Self {
				frames: right_frames,
				frame_index: 0,
				prev_index: u32::try_from(left_last_frame_index as i32 + self.index_shift).unwrap(),
				video_frame_index: u32::try_from(left_last_frame_index as i32 + self.index_shift + 1).unwrap(),
				index_shift: self.index_shift,
				end_iter: self.end_iter,
				eof_action: self.eof_action,
				last_video_frame: self.last_video_frame,
			};
			(left, right)
		} else {
			let right_frames = &self.frames[index..];
			let left = Self {
				// null iter
				frames: &[],
				frame_index: 0,
				prev_index: 0,
				video_frame_index: 0,
				index_shift: self.index_shift,
				end_iter: false,
				eof_action: self.eof_action,
				last_video_frame: self.last_video_frame,
			};
			let right = Self {
				frames: right_frames,
				frame_index: 0,
				prev_index: self.prev_index,
				video_frame_index: self.video_frame_index,
				index_shift: self.index_shift,
				end_iter: self.end_iter,
				eof_action: self.eof_action,
				last_video_frame: self.last_video_frame,
			};
			(left, right)
		};
		(left, right)
	}
}

#[cfg(test)]
mod tests {
	use derive_more::Deref;
	use rayon::iter::plumbing::Producer;
	use strum::IntoEnumIterator;

	use crate::osd::{FontVariant, Kind, TileIndices};

	use super::{
		EndOfFramesAction, ParallelVideoFramesRelIndexIter, SortedUniqFrames, VideoFramesRelIndexIter,
		VideoFramesRelIndexIterItem,
	};

	#[derive(PartialEq, Eq, Deref)]
	struct VideoFramesRelIndexIterItems<'a>(Vec<VideoFramesRelIndexIterItem<'a>>);

	impl VideoFramesRelIndexIterItems<'_> {
		fn display(&self) {
			println!("count: {}", self.len());
			println!("items: {self}");
		}
	}

	impl std::fmt::Display for VideoFramesRelIndexIterItems<'_> {
		fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
			use VideoFramesRelIndexIterItem::*;
			let string = self
				.iter()
				.map(|item| match item {
					Existing { rel_index, frame } => format!("e{rel_index}({})", frame.index()),
					FirstNonExisting => "c0".to_owned(),
					NonExisting {
						prev_rel_index: prev_index,
						rel_index: index,
					} => format!("l{index}:{prev_index}"),
				})
				.collect::<Vec<_>>()
				.join(" ");
			f.write_str(&string)
		}
	}

	impl std::fmt::Debug for VideoFramesRelIndexIterItems<'_> {
		fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
			f.write_str(&self.to_string())
		}
	}

	impl<'a> From<VideoFramesRelIndexIter<'a>> for ParallelVideoFramesRelIndexIter<'a> {
		fn from(vfrii: VideoFramesRelIndexIter<'a>) -> Self {
			Self {
				frames: vfrii.frames,
				frame_index: vfrii.frame_index,
				prev_index: vfrii.prev_index,
				video_frame_index: vfrii.video_frame_index,
				index_shift: vfrii.index_shift,
				end_iter: vfrii.end_iter,
				eof_action: vfrii.eof_action,
				last_video_frame: vfrii.last_video_frame,
			}
		}
	}

	#[test]
	fn split_video_frames_rel_index_iter() {
		let frames = [5, 8, 10, 11, 14].map(|index| super::Frame::new(index, TileIndices::new(vec![])));
		let frames = SortedUniqFrames::new(Kind::DJI_HD, FontVariant::Ardupilot, frames.to_vec());
		for eof_action in EndOfFramesAction::iter() {
			for first_video_frame in 0..15 {
				for last_video_frame in first_video_frame..18 {
					for video_frame_shift in -15..15 {
						let last_video_frame = if last_video_frame == 15 {
							None
						} else {
							Some(last_video_frame)
						};
						let frames_slice = frames.select_slice(first_video_frame, last_video_frame, video_frame_shift);
						let ref_items = VideoFramesRelIndexIterItems(
							frames_slice.video_frames_rel_index_iter(eof_action).collect::<Vec<_>>(),
						);
						ref_items.display();
						println!("----------------------------");
						for split in 0..frames_slice.len() {
							println!(
								"first_video_frame: {first_video_frame}, last_video_frame: {last_video_frame:?}, eof_action: {eof_action}, video_frame_shift: {video_frame_shift}, split: {split}"
							);
							let iter = ParallelVideoFramesRelIndexIter::from(
								frames_slice.video_frames_rel_index_iter(eof_action),
							);
							let (iter1, iter2) = iter.split_at(split);
							let (iter1, iter2) = (iter1.into_iter(), iter2.into_iter());
							let i1_items = VideoFramesRelIndexIterItems(iter1.collect::<Vec<_>>());
							let i2_items = VideoFramesRelIndexIterItems(iter2.collect::<Vec<_>>());
							i1_items.display();
							i2_items.display();

							// check that the items returned by the split iterator are the same as the non-split iterator
							let all_items = VideoFramesRelIndexIterItems(
								i1_items.0.into_iter().chain(i2_items.0.into_iter()).collect::<Vec<_>>(),
							);
							assert_eq!(ref_items, all_items);
							println!("****************************");
						}
					}
				}
			}
		}
	}
}
