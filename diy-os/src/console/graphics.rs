use super::font::{get_raster, SIZE, STYLE, WIDTH};

pub struct Pixels(pub usize);

#[derive(Clone, Copy)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl Color {
    pub const WHITE: Self = Self {
        red: 255,
        green: 255,
        blue: 255,
    };

    pub const BLACK: Self = Self {
        red: 0,
        green: 0,
        blue: 0,
    };

    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn scale(&mut self, intensity: u8) {
        if intensity == 0 {
            self.red = 0;
            self.green = 0;
            self.blue = 0;
        } else if intensity != 255 {
            self.red = (u16::from(self.red) * u16::from(intensity) / 255) as u8;

            self.green = (u16::from(self.green) * u16::from(intensity) / 255) as u8;

            self.blue = (u16::from(self.blue) * u16::from(intensity) / 255) as u8;
        }
    }
}

pub trait GraphicBackend {
    fn plot_pixel(&mut self, x: usize, y: usize, color: Color);

    fn fill(&mut self, color: Color);

    fn clear(&mut self) {
        self.fill(Color::BLACK);
    }

    fn get_x(&self) -> usize;
    fn get_y(&self) -> usize;
    fn set_x(&mut self, x: usize);
    fn set_y(&mut self, y: usize);
    fn get_width(&mut self) -> usize;
}

pub trait TextDrawer: GraphicBackend {
    fn new_line(&mut self);

    fn scroll(&mut self, amount: Pixels);

    fn draw_char(&mut self, char: char, color: Color) {
        let raster_char =
            get_raster(char, STYLE, SIZE).unwrap_or_else(|| get_raster(' ', STYLE, SIZE).unwrap());

        let mut new_x_pos: usize = self.get_x() + WIDTH;

        if new_x_pos > self.get_width() {
            self.new_line();
            new_x_pos = WIDTH;
        }

        if char == '\n' {
            self.new_line();
        } else {
            for (y, row) in raster_char.raster().iter().enumerate() {
                for (x, &byte) in row.iter().enumerate() {
                    if byte != 0 {
                        let mut pixel_color = color;
                        pixel_color.scale(byte);

                        self.plot_pixel(self.get_x() + x, self.get_y() + y, pixel_color);
                    }
                }
            }
            self.set_x(new_x_pos);
        }
    }

    fn draw_str(&mut self, str: &str, color: Color) {
        for c in str.chars() {
            self.draw_char(c, color);
        }
    }
}
