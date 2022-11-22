
use super::resolution::Resolution;

pub(crate) fn dimensions_diff(d1: Resolution, d2: Resolution) -> (i32, i32) {
    (d1.width as i32 - d2.width as i32, d1.height as i32 - d2.height as i32)
}

pub(crate) fn margins(outside_dimensions: Resolution, inside_dimensions: Resolution) -> (i32, i32) {
    let (margin_width_x2, margin_height_x2) = dimensions_diff(outside_dimensions, inside_dimensions);
    (margin_width_x2 / 2, margin_height_x2 / 2)
}
