mod png;
pub use self::png::{PngReadError, PngReader};

use std::io::Read;
use std::ops::Index;

pub use color::Color;
mod color {
    use std::fmt::{self, Display, Formatter, LowerHex, UpperHex};

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

    impl Display for Color {
        fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
            write!(
                fmt,
                "{}, {}, {}, {}",
                self.red, self.green, self.blue, self.alpha
            )
        }
    }

    impl LowerHex for Color {
        fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
            write!(
                fmt,
                "#{:02x}{:02x}{:02x}{:02x}",
                self.red, self.green, self.blue, self.alpha
            )
        }
    }

    impl UpperHex for Color {
        fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
            write!(
                fmt,
                "#{:02X}{:02X}{:02X}{:02X}",
                self.red, self.green, self.blue, self.alpha
            )
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

impl Index<(u32, u32)> for Image {
    type Output = Color;

    fn index(&self, (x, y): (u32, u32)) -> &Self::Output {
        assert!(
            x < self.width,
            "{} is larger than the image's width ({} px)",
            x,
            self.width
        );
        assert!(
            y < self.height,
            "{} is larger than the image's height ({} px)",
            y,
            self.height
        );
        // We can confidently cast to `usize`, since we know the dimensions are valid
        &self.pixels[(x + y * self.width) as usize]
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
