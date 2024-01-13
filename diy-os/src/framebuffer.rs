use crate::{
    console::{
        font::SIZE,
        graphics::{GraphicBackend, Pixels, TextDrawer},
    },
    spinlock::Spinlock,
};
use core::{fmt::Write, usize};

use bootloader_api::{
    info::{FrameBufferInfo, PixelFormat},
    BootInfo,
};

use crate::console::graphics::Color;

pub static FRAME_BUFER: Spinlock<Option<FrameBuffer>> = Spinlock::new(None);

pub fn init(mem: &'static mut [u8], info: bootloader_api::info::FrameBufferInfo) {
    let mut framebuffer = FrameBuffer::new(info, mem);
    framebuffer.clear();
    FRAME_BUFER.acquire().replace(framebuffer);
}

pub fn init_helper(boot_info: &'static mut BootInfo) {
    if let Some(framebuffer_field) = boot_info.framebuffer.as_mut() {
        let info = framebuffer_field.info();
        init(framebuffer_field.buffer_mut(), info);
    }
}

pub struct FrameBuffer {
    info: FrameBufferInfo,
    memio: &'static mut [u8], // the underlying memory mapped IO
    x: usize,                 // Current pixel in the x axis
    y: usize,                 // Current pixel in the y axis
}

impl FrameBuffer {
    pub fn new(info: FrameBufferInfo, memio: &'static mut [u8]) -> Self {
        Self {
            info,
            memio,
            x: 0,
            y: 0,
        }
    }
}

impl GraphicBackend for FrameBuffer {
    fn plot_pixel(&mut self, x: usize, y: usize, color: Color) {
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
            PixelFormat::Unknown {
                red_position,
                green_position,
                blue_position,
            } => {
                let buffer_ptr = self.memio.as_mut_ptr();

                let color_u32 = (u32::from(color.red) << red_position)
                    | (u32::from(color.blue) << blue_position)
                    | (u32::from(color.green) << green_position);

                unsafe {
                    *(buffer_ptr.byte_add(byte_offset).cast::<u32>()) = color_u32;
                }
            }
            _ => todo!(),
        }
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
        let number_of_bytes_to_copy = self.info.bytes_per_pixel * self.info.stride * amount.0;

        // copys amount number of pixels up
        (number_of_bytes_to_copy..self.memio.len())
            .enumerate()
            .for_each(|(counter, i)| {
                self.memio[counter] = self.memio[i];
            });

        // sets each pixel to black to clear the old pixels
        self.memio
            .iter_mut()
            .rev()
            .take(number_of_bytes_to_copy)
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
