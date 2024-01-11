use core::{usize, fmt::Write};

use bootloader_api::{info::{FrameBufferInfo, PixelFormat}, BootInfo};

pub static FRAME_BUFER: Spinlock<Option<FrameBuffer>> = Spinlock::new(None);

pub fn init(mem: &'static mut [u8], info: bootloader_api::info::FrameBufferInfo) {
    let mut framebuffer = FrameBuffer::new(info, mem);
    framebuffer.clear();
    FRAME_BUFER.acquire().replace(framebuffer);
}

pub fn init_helper(boot_info: &'static mut BootInfo) {
    if let Some(framebuffer_field) = boot_info.framebuffer.as_mut() {
        let info = framebuffer_field.info().clone();
        init(framebuffer_field.buffer_mut(), info);
    }
}

mod font {
    use noto_sans_mono_bitmap::{self, FontWeight, RasterHeight, RasterizedChar};

    pub const STYLE: FontWeight = FontWeight::Regular;
    pub const SIZE: RasterHeight = RasterHeight::Size16;
    pub const WIDTH: usize = get_raster_width(STYLE, SIZE);

    pub const fn get_raster_width(style: FontWeight, size: RasterHeight) -> usize {
        noto_sans_mono_bitmap::get_raster_width(style, size)
    }

    pub fn get_raster(c: char, style: FontWeight, size: RasterHeight) -> Option<RasterizedChar>{
        noto_sans_mono_bitmap::get_raster(c, style, size)
    }
}

pub mod api {
    use core::{usize, fmt::Write};

    use crate::framebuffer::font::{WIDTH, SIZE, STYLE, get_raster};

    pub struct Pixels(pub usize);

    #[derive(Clone, Copy)]
    pub struct Color {
        pub red: u8,
        pub green: u8,
        pub blue: u8,
    }

    impl Color {
        fn new(red: u8, green: u8, blue: u8) -> Self {
            Self {
                red,
                green,
                blue,
            }
        }

        fn scale(&mut self, intensity: u8) {
            let mut temp: u16;

            temp = u16::from(self.blue) * u16::from(intensity);

            self.blue = (temp / 255) as u8;

            temp = u16::from(self.green) * u16::from(intensity);

            self.green = (temp / 255) as u8;

            temp = u16::from(self.red) * u16::from(intensity);

            self.red = (temp / 255) as u8;
        }
    }

    pub trait GraphicBackend {
        fn plot_pixel(&mut self, x: usize, y: usize, color: Color);

        fn fill(&mut self, color: Color);

        fn clear(&mut self) {
            let black = Color::new(0, 0, 0);
            self.fill(black);
        }

        fn get_x(&self) -> usize;
        fn get_y(&self) -> usize;
        fn set_x(&mut self, x: usize);
        fn set_y(&mut self, y: usize);
        fn get_width(&mut self) -> usize;
    }

    // impl<T: GraphicBackend> TextDrawer for T { }

    pub trait TextDrawer : GraphicBackend {
        fn new_line(&mut self);

        fn scroll(&mut self, amount: Pixels);

        fn draw_char(&mut self, char: char, color: Color) {
            let raster_char = get_raster(char, STYLE, SIZE).unwrap_or(get_raster(' ', STYLE, SIZE).unwrap());
            
            let mut new_x_pos: usize = self.get_x() + WIDTH;

            if new_x_pos > self.get_width() {
                self.new_line();
                new_x_pos = WIDTH;
            }

            if char != '\n' {
                for (y, row) in raster_char.raster().iter().enumerate() {
                    for (x, &byte) in row.iter().enumerate() {
                        if byte != 0 {
                            let mut color = color.clone();
                            color.scale(byte);

                            self.plot_pixel(self.get_x() + x, self.get_y() + y, color);
                        }
                    }
                }
                self.set_x(new_x_pos);
            } else {
                self.new_line();
            }
        }

        fn draw_str(&mut self, str: &str, color: Color) {
            for c in str.chars() {
                self.draw_char(c, color);
            }
        }
    }

}

pub struct FrameBuffer {
    info: FrameBufferInfo,
    memio: &'static mut[u8], // the underlying memory mapped IO
    x: usize, // Current pixel in the x axis
    y: usize, // Current pixel in the y axis
}

impl FrameBuffer {
    pub fn new(info: FrameBufferInfo, memio: &'static mut[u8]) -> Self {
        Self { 
            info,
            memio,
            x: 0, 
            y: 0,
        }
    }
}

use api::GraphicBackend;

use crate::spinlock::Spinlock;

use self::{api::{TextDrawer, Pixels}, font::SIZE};

impl GraphicBackend for FrameBuffer {

    fn plot_pixel(&mut self, x: usize, y: usize, color: api::Color) {
        let byte_offset = self.info.bytes_per_pixel * (y * self.info.stride + x);

        match self.info.pixel_format {
            PixelFormat::Bgr => {
                self.memio[byte_offset] = color.blue;
                self.memio[byte_offset + 1] = color.green;
                self.memio[byte_offset + 2] = color.red;
            }
            PixelFormat::Rgb => {
                self.memio[byte_offset] = color.red;
                self.memio[byte_offset + 1] = color.green;
                self.memio[byte_offset + 2] = color.blue;
            }
            PixelFormat::Unknown { red_position, green_position, blue_position } => {

                let ptr = self.memio.as_mut_ptr();

                let color_u32 = ((color.red as u32) << red_position) | ((color.blue as u32) << blue_position) | ((color.green as u32) << green_position);

                unsafe {
                    *(ptr.byte_offset(byte_offset as isize) as *mut u32) = color_u32;
                }

            },
            _ => todo!(),
        }
    }

    /// TODO: Write an effient implemation that does not recalc
    /// https://wiki.osdev.org/Drawing_In_Protected_Mode/
    /// see optimization
    fn fill(&mut self, color: api::Color) {
        for y in 0..self.info.height {
            for x in 0..self.info.width {
                self.plot_pixel(x, y, color);
            }
        }
    }

    fn get_x(&self) -> usize {
        self.x
    }

    fn get_y(&self) -> usize {
        self.y
    }

    fn set_x(&mut self, x: usize) {
        self.x = x;
    }

    fn set_y(&mut self, y: usize) {
        self.y = y;
    }

    fn get_width(&mut self) -> usize {
        self.info.width
    }
}


impl TextDrawer for FrameBuffer {
    fn new_line(&mut self) {
        self.x = 0;

        let new_y = self.y + SIZE.val();
        if new_y >= self.info.height {
            self.scroll(Pixels(SIZE.val()));
        } else {
            self.y = new_y;
        }
    }

    fn scroll(&mut self, amount: Pixels) {
        // copy memory one row up
        // for i in 1..self.info.height {
        //     for j in 1..self.info.width {
        //         let byte_offset = self.info.bytes_per_pixel * (j * self.info.stride + i);
        //         serial_println!("{}", byte_offset);
        //
        //         // let temp = self.memio[byte_offset];
        //         // self.memio[byte_offset -1] = temp;
        //     }
        // }

        let mut counter: usize = 0;
        for i in (self.info.bytes_per_pixel * self.info.stride * amount.0)..self.memio.len() {
            self.memio[counter] = self.memio[i];
            counter += 1;
        }

        for i in self.memio.iter_mut().skip(self.info.bytes_per_pixel * self.info.stride * (self.info.height - amount.0)) {
            *i = 0;
        }

        // self.y -= amount;

        // fill bottom row with black
        // for i in (self.info.height - SIZE.val())..self.info.height {
        //     for j in 0..self.info.width {
        //         self.plot_pixel(i, j, api::Color { red: 0, green: 0, blue: 0, });
        //     }
        // }
    }
}

impl Write for FrameBuffer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.draw_str(s, api::Color { red: 255, green: 255, blue: 255});
        Ok(())
    }
}


// mod temp {
// pub trait PixelFormat {
//     fn scale(&self, intensity: u8) -> Self;
//     fn new(red: u8, green: u8, blue: u8) -> Self;
// }
//
// #[repr(C)]
// #[derive(Clone, Copy)]
// pub struct Bgr {
//     pub blue: u8,
//     pub green: u8,
//     pub red: u8,
//     _reserved: u8,
// }
//
// impl Bgr {
//     pub fn new(blue: u8, green: u8, red: u8) -> Self {
//         Self {
//             blue,
//             green,
//             red,
//             _reserved: 0,
//         }
//     }
// } 
//
// impl PixelFormat for Bgr {
//     fn scale(&self, intensity: u8) -> Self {
//         let mut copy = self.clone();
//
//         let mut temp: u16;
//
//         temp = u16::from(copy.blue) * u16::from(intensity);
//
//         copy.blue = (temp / 255) as u8;
//
//         temp = u16::from(copy.green) * u16::from(intensity);
//
//         copy.green = (temp / 255) as u8;
//
//         temp = u16::from(copy.red) * u16::from(intensity);
//
//         copy.red = (temp / 255) as u8;
//
//         copy
//     }
//
//     fn new(red: u8, green: u8, blue: u8) -> Self {
//         Bgr::new(blue, green, red)
//     }
// }
//
// #[repr(C)]
// #[derive(Clone, Copy)]
// pub struct Rgb {
//     pub red: u8,
//     pub green: u8,
//     pub blue: u8,
//     _reserved: u8,
// }
//
// impl Rgb {
//     pub fn new(red: u8, green: u8, blue: u8) -> Self {
//         Self {
//             red,
//             green,
//             blue,
//             _reserved: 0,
//         }
//     }
// } 
//
// impl PixelFormat for Rgb {
//     fn scale(&self, intensity: u8) -> Self {
//         let mut copy = self.clone();
//
//         copy.blue = copy.blue * intensity / 255;
//         copy.green = copy.green * intensity / 255;
//         copy.red = copy.red * intensity / 255;
//
//         copy
//     }
//
//     fn new(red: u8, green: u8, blue: u8) -> Self {
//         Rgb::new(red, green, blue)
//     }
// }
//
// pub struct FrameBuffer {
//     buffer: &'static mut [u8],
//     info: FrameBufferInfo,
//     x_pos: usize,
//     y_pos: usize,
// }
//
// impl FrameBuffer {
//     pub fn new(buffer: &'static mut [u8], info: FrameBufferInfo) -> Self {
//         Self {
//             buffer,
//             info,
//             x_pos: 0,
//             y_pos: 0,
//         }
//     }
//
//     pub fn write_pixel<T: PixelFormat>(&mut self, x: usize, y: usize, color: T) {
//         let pixel_offset = y * self.info.stride + x;
//
//         unsafe { mem::transmute::<&mut [u8], &mut [T]>(self.buffer)[pixel_offset] = color; };
//     }
//
// }
//
// impl FrameBuffer {
//     pub fn clear<T: PixelFormat + Copy>(&mut self) {
//         let black = T::new(0, 0, 0);
//         self.fill(black);
//     }
//
//     pub fn fill<T: PixelFormat + Copy>(&mut self, color: T) {
//         // self.buffer.fill(color);
//         let color_u8 = unsafe { transmute_unchecked::<T, u8>(color) };
//
//         for i in self.buffer.iter_mut().skip(4) {
//             *i = color_u8;
//         }
//     }
//
//     pub fn draw_square<T: PixelFormat + Copy>(&mut self, pos_x: usize, pos_y: usize, width: usize, length: usize, color: T) {
//         for y in pos_x..pos_x + width {
//             for x in pos_y..pos_y + length {
//                 self.write_pixel(x, y, color);
//             }
//         }
//     }
//
//     pub fn draw_char<T: PixelFormat + Copy>(&mut self, char: char, color: T) {
//         let raster_char = get_raster(char, STYLE, SIZE).unwrap_or(get_raster(' ', STYLE, SIZE).unwrap());
//
//         let new_x_pos = self.x_pos + WIDTH;
//         let new_y_pos = self.y_pos + SIZE.val();
//
//         for (y, row) in raster_char.raster().iter().enumerate() {
//             for (x, &byte) in row.iter().enumerate() {
//                 if byte != 0 {
//                     self.write_pixel(self.x_pos + x, self.y_pos + y, color.scale(byte))
//                 }
//             }
//         }
//         self.x_pos = new_x_pos;
//     }
//
//     pub fn draw_str<T: PixelFormat + Copy>(&mut self, str: &str, color: T) {
//         for c in str.chars() {
//             self.draw_char(c, color);
//         }
//     }
// }
//
// // impl<T: PixelFormat + Copy> Write for FrameBuffer {
// //     fn write_str(&mut self, s: &str) -> core::fmt::Result {
// //         self.draw_str(s, T::new(255, 255, 255));
// //         Ok(())
// //     }
// // }
//
//
// const STYLE: FontWeight = FontWeight::Regular;
// const SIZE: RasterHeight = RasterHeight::Size16;
// const WIDTH: usize = get_raster_width(STYLE, SIZE);
//
// trait Drawer {
// }
// }
