mod png;
pub use self::png::{PngReadError, PngReader};

use std::io::Read;
use std::ops::Index;

pub use color::Color;
mod color {
    use std::fmt::{self, Display, Formatter, LowerHex, UpperHex};

    // Implementing `PartialEq` in this way makes identical colors with a different palette index
    // different. This is intentional, so that a palette may contain duplicates of a given color
    // if the user insists on it (either via a PNG palette, or a CLI specification)
    #[derive(Debug, PartialEq, Eq, Clone)]
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

        pub fn rgba(&self) -> [u8; 4] {
            [self.red, self.green, self.blue, self.alpha]
        }

        pub fn from_rgb555(color: u16, index: Option<u8>) -> Self {
            Self::new(
                Self::rgb_to_rgba((
                    color as u8 & 0x1F,
                    (color >> 5) as u8 & 0x1F,
                    (color >> 10) as u8 & 0x1F,
                )),
                index,
            )
        }

        pub fn to_rgb555(&self) -> u16 {
            u16::from(self.red) | u16::from(self.green) << 5 | u16::from(self.blue) << 10
        }

        pub fn rgb_to_rgba((red, green, blue): (u8, u8, u8)) -> (u8, u8, u8, u8) {
            (red, green, blue, 255)
        }

        pub fn gray_to_rgb(gray: u8) -> (u8, u8, u8) {
            (gray, gray, gray)
        }

        pub fn luma_chroma(&self) -> (f32, f32, f32) {
            let (red, green, blue) = (
                f32::from(self.red),
                f32::from(self.green),
                f32::from(self.blue),
            );
            // Luminance, as defined by CCIR 601
            let luma = 0.299 * red + 0.587 * green + 0.114 * blue;
            (luma, blue - luma, red - luma)
        }

        pub fn distance(&self, rhs: &Color) -> u8 {
            // Get YUV (luma, blue chroma, red chroma) for both sides
            let (ly, lu, lv) = self.luma_chroma();
            let (ry, ru, rv) = rhs.luma_chroma();
            let dist = (((ly - ry).powi(2) + (lu - ru).powi(2) + (lv - rv).powi(2)) / 3.0)
                .sqrt()
                .round();
            assert!(
                (0.0..=255.0).contains(&dist),
                "Color distance not in u8 range! ({})",
                dist
            );
            dist as u8
        }
    }

    impl Default for Color {
        fn default() -> Self {
            Self::new((0, 0, 0, 255), None)
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
