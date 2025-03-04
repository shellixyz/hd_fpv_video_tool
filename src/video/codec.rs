use strum::EnumIter;

use crate::prelude::OverlayVideoCodec;

#[derive(Debug, Clone, Copy, PartialEq, Eq, derive_more::derive::Display, EnumIter)]
pub enum Codec {
	AV1,
	H264,
	H265,
	VP8,
	VP9,
}

impl Codec {
	pub fn ffmpeg_string(&self, hw_accel: bool) -> &'static str {
		match hw_accel {
			true => match self {
				Self::AV1 => "av1_vaapi",
				Self::H264 => "h264_vaapi",
				Self::H265 => "hevc_vaapi",
				Self::VP8 => "vp8_vaapi",
				Self::VP9 => "vp9_vaapi",
			},
			false => match self {
				Self::AV1 => "libaom-av1",
				Self::H264 => "libx264",
				Self::H265 => "libx265",
				Self::VP8 => "libvpx",
				Self::VP9 => "libvpx-vp9",
			},
		}
	}
}

impl From<OverlayVideoCodec> for Codec {
	fn from(codec: OverlayVideoCodec) -> Self {
		match codec {
			OverlayVideoCodec::VP8 => Self::VP8,
			OverlayVideoCodec::VP9 => Self::VP9,
			OverlayVideoCodec::HEVC => Self::H265,
		}
	}
}
