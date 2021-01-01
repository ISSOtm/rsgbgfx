mod png;
pub use self::png::{PngReadError, PngReader};

use std::io::Read;

pub use color::Color;
mod color {
    #[derive(Debug)]
    pub struct Color {
        red: u8,
        green: u8,
        blue: u8,
        alpha: u8,
        palette_index: Option<u8>,
    }

    impl Color {
        pub fn new((red, green, blue, alpha): (u8, u8, u8, u8), index: Option<u8>) -> Self {
            Self {
                red,
                green,
                blue,
                alpha,
                palette_index: index,
            }
        }

        pub fn rgb_to_rgba((red, green, blue): (u8, u8, u8)) -> (u8, u8, u8, u8) {
            (red, green, blue, 255)
        }

        pub fn gray_to_rgb(gray: u8) -> (u8, u8, u8) {
            (gray, gray, gray)
        }
    }
}

#[derive(Debug)]
pub struct Image {
    width: u32,  // Size in pixels
    height: u32, // Size in pixels
    pixels: Vec<Color>,
}

impl Image {
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }
}

pub trait ImageReader<R: Read> {
    type NewError;
    fn new(input: R) -> Result<Self, Self::NewError>
    where
        Self: Sized;
    type ReadError;
    fn read_image(&mut self) -> Result<Image, Self::ReadError>;
}
