
use super::file::Frame as OSDFileFrame;
use image::{ImageBuffer, Rgba, GenericImage};
use hd_fpv_osd_font_tool::osd::{bin_file::BinFileReader as BinFontFileReader, standard_size_tile_container::StandardSizeTileArray};

type Frame = ImageBuffer<Rgba<u8>, Vec<u8>>;

// pub fn draw_frame(osd_file_frame: &OSDFileFrame) -> Frame {
//     let font_tiles = BinFontFileReader::open("../hd_fpv_osd_font_tool/font_files/font_hd.bin").unwrap().tile_array().unwrap();
//     let mut image = Frame::new(60 * 24, 22 * 36);

//     for (tile_index, tile_value) in osd_file_frame.data().iter().enumerate() {
//         let (tile_x, tile_y) = (tile_index / 22, tile_index % 22);
//         if *tile_value != 0 {
//             image.copy_from(font_tiles[*tile_value as usize].image(), tile_x as u32 * 24, tile_y as u32 * 36).unwrap();
//         }
//     }

//     image
// }

pub struct Generator {
    font_tiles : StandardSizeTileArray
}

impl Generator {

    pub fn new() -> Self {
        let font_tiles = BinFontFileReader::open("../hd_fpv_osd_font_tool/font_files/font_hd.bin").unwrap().tile_array().unwrap();
        Self { font_tiles }
    }

    pub fn draw_frame(&self, osd_file_frame: &OSDFileFrame) -> Frame {
        let mut image = Frame::new(60 * 24, 22 * 36);

        for (tile_index, tile_value) in osd_file_frame.data().iter().enumerate() {
            let (tile_x, tile_y) = (tile_index / 22, tile_index % 22);
            if *tile_value != 0 {
                image.copy_from(self.font_tiles[*tile_value as usize].image(), tile_x as u32 * 24, tile_y as u32 * 36).unwrap();
            }
        }

        image
    }

}