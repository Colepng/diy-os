use noto_sans_mono_bitmap::{self, FontWeight, RasterHeight, RasterizedChar};

pub const STYLE: FontWeight = FontWeight::Regular;
pub const SIZE: RasterHeight = RasterHeight::Size16;
pub const WIDTH: usize = get_raster_width(STYLE, SIZE);

pub const fn get_raster_width(style: FontWeight, size: RasterHeight) -> usize {
    noto_sans_mono_bitmap::get_raster_width(style, size)
}

pub fn get_raster(c: char, style: FontWeight, size: RasterHeight) -> Option<RasterizedChar> {
    noto_sans_mono_bitmap::get_raster(c, style, size)
}
