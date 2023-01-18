
pub mod file;

pub const AU_OSD_FRAME_SHIFT: i32 = -36;

pub mod dimensions {
    use crate::osd::Dimensions;
    pub const SD: Dimensions = Dimensions::new(30, 15);
    pub const FAKE_HD: Dimensions = Dimensions::new(60, 22);
    pub const HD: Dimensions = Dimensions::new(50, 18);
}
