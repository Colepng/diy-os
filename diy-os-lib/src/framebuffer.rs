use crate::console::{
    font::SIZE,
    graphics::{GraphicBackend, Pixels, TextDrawer},
};
use core::{
    fmt::Write,
    mem::{Assume, TransmuteFrom},
};

use volatile::VolatileRef;

use spinlock::Spinlock;

use bootloader_api::info::{FrameBufferInfo, PixelFormat};

use crate::console::graphics::Color;

pub static FRAME_BUFER: Spinlock<Option<FrameBuffer>> = Spinlock::new(None);

pub fn init(framebuffer_bootinfo: bootloader_api::info::FrameBuffer) {
    let info = framebuffer_bootinfo.info();
    let mem = framebuffer_bootinfo.into_buffer();
    let bytes_fn = match info.pixel_format {
        PixelFormat::Rgb => |_, color: Color| (color.red, color.green, color.blue, 0),
        PixelFormat::Bgr => |_, color: Color| (color.blue, color.green, color.red, 0),
        PixelFormat::U8 => todo!(),
        PixelFormat::Unknown {
            red_position: _,
            green_position: _,
            blue_position: _,
        } => |pixel_format, color: Color| {
            if let PixelFormat::Unknown {
                red_position,
                green_position,
                blue_position,
            } = pixel_format
            {
                let color_u32 = (u32::from(color.red) << red_position)
                    | (u32::from(color.blue) << blue_position)
                    | (u32::from(color.green) << green_position);

                unsafe {
                    <(u8, u8, u8, u8) as TransmuteFrom<u32, { Assume::NOTHING }>>::transmute(
                        color_u32,
                    )
                }
            } else {
                unreachable!()
            }
        },
        _ => todo!(),
    };
    let mut framebuffer = FrameBuffer::new(info, mem, bytes_fn);

    framebuffer.clear();
    FRAME_BUFER.acquire().replace(framebuffer);
}

pub struct FrameBuffer {
    info: FrameBufferInfo,             // Info about the frame buffer
    memio: VolatileRef<'static, [u8]>, // the underlying memory mapped IO
    x: usize,                          // Current pixel in the x axis
    y: usize,                          // Current pixel in the y axis
    bytes_fn: fn(PixelFormat, Color) -> (u8, u8, u8, u8), // Converts the colors to 4 u8s
}

impl FrameBuffer {
    pub fn new(
        info: FrameBufferInfo,
        memio: &'static mut [u8],
        bytes_fn: fn(PixelFormat, Color) -> (u8, u8, u8, u8),
    ) -> Self {
        Self {
            info,
            memio: unsafe { VolatileRef::new(memio.into()) },
            x: 0,
            y: 0,
            bytes_fn,
        }
    }

    fn write_pixel(&mut self, byte_offset: usize, color: Color) {
        let bytes = (self.bytes_fn)(self.info.pixel_format, color);
        let ptr = self.memio.as_mut_ptr();
        ptr.index(byte_offset).write(bytes.0);
        ptr.index(byte_offset + 1).write(bytes.1);
        ptr.index(byte_offset + 2).write(bytes.2);
        ptr.index(byte_offset + 3).write(bytes.3);
    }
}

impl GraphicBackend for FrameBuffer {
    fn plot_pixel(&mut self, x: usize, y: usize, color: Color) {
        let byte_offset = self.info.bytes_per_pixel * (y * self.info.stride + x);

        self.write_pixel(byte_offset, color);
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
        memio_ptr.copy_within(number_of_bytes_to_scroll..self.info.byte_len, 0);

        // sets each pixel to black to clear the old pixels
        memio_ptr
            .iter()
            .skip(self.info.byte_len - number_of_bytes_to_scroll)
            .for_each(|byte| {
                byte.write(0);
            });
    }
}

impl Write for FrameBuffer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.draw_str(s, Color::WHITE);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use bootloader_api::info::PixelFormat;

    use super::{Assume, Color, FrameBuffer, FrameBufferInfo, TransmuteFrom};
    use crate::console::graphics::GraphicBackend;

    fn init(format: PixelFormat) -> FrameBuffer {
        static mut BUFFER: [u8; 4 * 100 * 100] = [0; 4 * 100 * 100];

        let info = FrameBufferInfo {
            byte_len: 4 * 100 * 100,
            width: 100,
            height: 100,
            pixel_format: format,
            bytes_per_pixel: 4,
            stride: 100,
        };

        let mem = unsafe { &mut *&raw mut BUFFER };
        let bytes_fn = match format {
            PixelFormat::Rgb => |_, color: Color| (color.red, color.green, color.blue, 0),
            PixelFormat::Bgr => |_, color: Color| (color.blue, color.green, color.red, 0),
            PixelFormat::U8 => todo!(),
            PixelFormat::Unknown {
                red_position: _,
                green_position: _,
                blue_position: _,
            } => |pixel_format, color: Color| {
                if let PixelFormat::Unknown {
                    red_position,
                    green_position,
                    blue_position,
                } = pixel_format
                {
                    let color_u32 = (u32::from(color.red) << red_position)
                        | (u32::from(color.blue) << blue_position)
                        | (u32::from(color.green) << green_position);

                    unsafe {
                        <(u8, u8, u8, u8) as TransmuteFrom<u32, { Assume::NOTHING }>>::transmute(
                            color_u32,
                        )
                    }
                } else {
                    unreachable!()
                }
            },
            _ => todo!(),
        };
        let mut framebuffer = FrameBuffer::new(info, mem, bytes_fn);

        framebuffer.clear();
        framebuffer
    }

    #[test]
    fn plotting_bgr_test() {
        let mut fb = init(PixelFormat::Bgr);

        let color = crate::console::graphics::Color {
            red: 255,
            green: 255,
            blue: 100,
        };

        fb.plot_pixel(0, 0, color);

        assert_eq!(fb.memio.as_ptr().index(2).read(), 255);
        assert_eq!(fb.memio.as_ptr().index(1).read(), 255);
        assert_eq!(fb.memio.as_ptr().index(0).read(), 100);
    }
    #[test]
    fn plotting_rgb_test() {
        let mut fb = init(PixelFormat::Rgb);

        let color = crate::console::graphics::Color {
            red: 255,
            green: 255,
            blue: 100,
        };

        fb.plot_pixel(0, 0, color);

        assert_eq!(fb.memio.as_ptr().index(0).read(), 255);
        assert_eq!(fb.memio.as_ptr().index(1).read(), 255);
        assert_eq!(fb.memio.as_ptr().index(2).read(), 100);
    }

    #[test]
    fn plotting_unknown_test() {
        let mut fb = init(PixelFormat::Unknown {
            red_position: 16,
            green_position: 8,
            blue_position: 0,
        });

        let color = crate::console::graphics::Color {
            red: 255,
            green: 255,
            blue: 100,
        };

        fb.plot_pixel(0, 0, color);

        assert_eq!(fb.memio.as_ptr().index(2).read(), 255);
        assert_eq!(fb.memio.as_ptr().index(1).read(), 255);
        assert_eq!(fb.memio.as_ptr().index(0).read(), 100);
    }
}
