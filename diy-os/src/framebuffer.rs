use crate::{
    console::{
        font::SIZE,
        graphics::{GraphicBackend, Pixels, TextDrawer},
    },
    spinlock::Spinlock,
};
use core::fmt::Write;

use bootloader_api::info::{FrameBufferInfo, PixelFormat};

use crate::console::graphics::Color;

pub static FRAME_BUFER: Spinlock<Option<FrameBuffer>> = Spinlock::new(None);

pub fn init(framebuffer_bootinfo: bootloader_api::info::FrameBuffer) {
    let info = framebuffer_bootinfo.info();
    let mem = framebuffer_bootinfo.into_buffer();
    let mut framebuffer = FrameBuffer::new(info, mem);
    framebuffer.clear();
    FRAME_BUFER.acquire().replace(framebuffer);
}

pub struct FrameBuffer {
    info: FrameBufferInfo,    // Info about the frame buffer
    memio: &'static mut [u8], // the underlying memory mapped IO
    x: usize,                 // Current pixel in the x axis
    y: usize,                 // Current pixel in the y axis
    write_pixel_fn: fn(&mut FrameBuffer, usize, Color), // Function pointer to the function that
                              // writes the appropriate pixel format
}

impl FrameBuffer {
    pub fn new(info: FrameBufferInfo, memio: &'static mut [u8]) -> Self {
        Self {
            info,
            memio,
            x: 0,
            y: 0,
            write_pixel_fn: match info.pixel_format {
                PixelFormat::Bgr => Self::write_bgr_pixel,
                PixelFormat::Rgb => Self::write_rgb_pixel,
                PixelFormat::Unknown { .. } => Self::write_unknown_4byte_pixel,
                _ => todo!(),
            },
        }
    }

    fn write_bgr_pixel(&mut self, byte_offset: usize, color: Color) {
        self.memio[byte_offset] = color.blue;
        self.memio[byte_offset + 1] = color.green;
        self.memio[byte_offset + 2] = color.red;
    }

    fn write_rgb_pixel(&mut self, byte_offset: usize, color: Color) {
        self.memio[byte_offset] = color.blue;
        self.memio[byte_offset + 1] = color.green;
        self.memio[byte_offset + 2] = color.red;
    }

    fn write_unknown_4byte_pixel(&mut self, byte_offset: usize, color: Color) {
        if let PixelFormat::Unknown {
            red_position,
            green_position,
            blue_position,
        } = self.info.pixel_format
        {
            let buffer_ptr = self.memio.as_mut_ptr();

            let color_u32 = (u32::from(color.red) << red_position)
                | (u32::from(color.blue) << blue_position)
                | (u32::from(color.green) << green_position);

            // SAFETY: Assuming x and y are not larger then the screen size byte_offset should
            // never be larger then the number of available bytes
            let pixel_ptr = unsafe { buffer_ptr.byte_add(byte_offset) };

            #[allow(clippy::cast_ptr_alignment)]
            // SAFETY: The pointer is aligned to a 4 byte boundary since there is 4
            // bytes per pixel
            unsafe {
                *pixel_ptr.cast::<u32>() = color_u32;
            }
        }
    }
}

impl GraphicBackend for FrameBuffer {
    fn plot_pixel(&mut self, x: usize, y: usize, color: Color) {
        let byte_offset = self.info.bytes_per_pixel * (y * self.info.stride + x);

        (self.write_pixel_fn)(self, byte_offset, color);
    }

    // TODO: Write an effient implemation that does not recalc
    // https://wiki.osdev.org/Drawing_In_Protected_Mode
    // see optimization
    fn fill(&mut self, color: Color) {
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
        let number_of_bytes_to_scroll = self.info.bytes_per_pixel * self.info.stride * amount.0;

        let memio_ptr = self.memio.as_mut_ptr();

        // shifts up everything but the last amount of pixels
        unsafe {
            core::ptr::copy(
                memio_ptr.byte_add(number_of_bytes_to_scroll),
                memio_ptr,
                self.info.byte_len - number_of_bytes_to_scroll,
            );
        }

        // sets each pixel to black to clear the old pixels
        self.memio
            .iter_mut()
            .rev()
            .take(number_of_bytes_to_scroll)
            .for_each(|byte| {
                *byte = 0;
            });
    }
}

impl Write for FrameBuffer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.draw_str(s, Color::WHITE);
        Ok(())
    }
}
