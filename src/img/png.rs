use super::{Color, Image, ImageReader};
use png::{ColorType, Decoder, DecodingError, Reader, Transformations};
use std::convert::{TryFrom, TryInto};
use std::error;
use std::fmt::Display;
use std::fmt::{self, Formatter};
use std::io::Read;

pub struct PngReader<R: Read> {
    reader: Reader<R>,
}

impl<R: Read> ImageReader<R> for PngReader<R> {
    type NewError = DecodingError;

    fn new(input: R) -> Result<Self, Self::NewError> {
        let mut decoder = Decoder::new(input);
        decoder.set_transformations(Transformations::IDENTITY);
        let (_, reader) = decoder.read_info()?;
        Ok(Self { reader })
    }

    type ReadError = PngReadError;

    fn read_image(&mut self) -> Result<Image, Self::ReadError> {
        let info = self.reader.info();
        let (width, height, color_type, bit_depth) =
            (info.width, info.height, info.color_type, info.bit_depth);

        // Compute the palette, if any.
        // `info.palette` contains the PLTE chunk, which contains RGB888 entries, if present.
        // `info.trns` contains the tRNS chunks, which contains 8-bit alpha entries, if present.
        let palette = info.palette.as_ref().map(|buf| {
            assert_eq!(buf.len() % 3, 0);
            let nb_colors = buf.len() / 3;
            let mut palette = Vec::with_capacity(nb_colors);
            let mut it = buf.iter();
            for i in 0..nb_colors {
                let (r, g, b) = (
                    *it.next().unwrap(),
                    *it.next().unwrap(),
                    *it.next().unwrap(),
                );
                palette.push(
                    info.trns
                        .as_ref()
                        .map_or_else(|| Color::rgb_to_rgba((r, g, b)), |trns| (r, g, b, trns[i])),
                );
            }
            palette
        });

        let nb_pixels: usize = width
            .checked_mul(height)
            .map(|size| size.try_into().ok()) // We don't care about the actual error
            .flatten() // Return the same error whether the multiplication or conversion failed
            .ok_or(PngReadError::TooBig(width, height))?;

        let mut data = vec![0; info.raw_bytes()];
        self.reader
            .next_frame(&mut data)
            .map_err(PngReadError::DecodingError)?;
        let mut samples = SampleIterator::new(&data, bit_depth, width);

        // Write pixels from raw data
        let mut pixels = Vec::with_capacity(nb_pixels);
        for _ in 0..nb_pixels {
            pixels.push(match color_type {
                ColorType::Grayscale => {
                    assert_eq!(color_type.samples(), 1);
                    Color::new(
                        Color::rgb_to_rgba(Color::gray_to_rgb(samples.next().unwrap())),
                        None,
                    )
                }
                ColorType::RGB => {
                    assert_eq!(color_type.samples(), 3);
                    Color::new(
                        Color::rgb_to_rgba((
                            samples.next().unwrap(),
                            samples.next().unwrap(),
                            samples.next().unwrap(),
                        )),
                        None,
                    )
                }
                ColorType::Indexed => {
                    assert_eq!(color_type.samples(), 1);
                    let index = samples.next().unwrap();
                    let palette = palette.as_ref().unwrap();
                    Color::new(palette[usize::try_from(index).unwrap()], Some(index))
                }
                ColorType::GrayscaleAlpha => {
                    assert_eq!(color_type.samples(), 2);
                    let rgb = Color::gray_to_rgb(samples.next().unwrap());
                    Color::new((rgb.0, rgb.1, rgb.2, samples.next().unwrap()), None)
                }
                ColorType::RGBA => {
                    assert_eq!(color_type.samples(), 4);
                    Color::new(
                        (
                            samples.next().unwrap(),
                            samples.next().unwrap(),
                            samples.next().unwrap(),
                            samples.next().unwrap(),
                        ),
                        None,
                    )
                }
            });
        }

        Ok(Image {
            width,
            height,
            pixels,
        })
    }
}

use iter::SampleIterator;
mod iter {
    use png::BitDepth;

    #[derive(Debug)]
    pub struct SampleIterator<'a> {
        buf: &'a [u8],
        bit_depth: BitDepth,
        width: u32,

        x: u32,
        index: usize,
        shift: u8,
    }

    impl<'a> SampleIterator<'a> {
        pub fn new(buf: &'a [u8], bit_depth: BitDepth, width: u32) -> Self {
            Self {
                buf,
                bit_depth,
                width,
                x: 0,
                index: 0,
                shift: 8,
            }
        }
    }

    // Iterator over a PNG's samples
    // Note that the logic may be more convoluted for images smaller than 8 pixels
    // (Try a 1-bit colormap with less than 8 pixels, you'll see)
    // We don't attempt to handle those cases, since we require images to be at least 8 pixels
    impl Iterator for SampleIterator<'_> {
        type Item = u8;

        fn next(&mut self) -> Option<<Self as Iterator>::Item> {
            if self.index == self.buf.len() {
                None
            } else {
                use BitDepth::*;

                Some(if self.bit_depth == Sixteen {
                    // 16-bit needs special handling.
                    // Its entries are larger than `u8`, so we perform truncation;
                    // such precision isn't needed for a Game Boy game, anyway.
                    // There is one exception, which is palette indexes...
                    // However, Section 11.2.2 of the PNG standard disallows several
                    // combinations of bit depth and color type, including this one!
                    let val = self.buf[self.index]; // `png` returns big-endian bytes
                    self.index += 1;
                    // Check for an odd-sized buffer; it would be incorrect, but let's be lenient
                    if self.index != self.buf.len() {
                        self.index += 1; // Ignore the second byte
                    }
                    val
                } else {
                    let len = match self.bit_depth {
                        One => 1,
                        Two => 2,
                        Four => 4,
                        Eight => 8,
                        Sixteen => unreachable!(), // Handled above
                    };

                    let mask = (1 << len) - 1;
                    self.shift -= len;
                    let val = self.buf[self.index] >> self.shift & mask;

                    // If finished reading a byte, or at scanline end, reset and go to the next one
                    self.x += 1;
                    if self.x == self.width {
                        self.x = 0;
                        self.shift = 0; // Force going to the next byte
                    }
                    if self.shift == 0 {
                        self.index += 1;
                        self.shift = 8;
                    }

                    val
                })
            }
        }
    }
}

#[derive(Debug)]
pub enum PngReadError {
    DecodingError(DecodingError),
    TooBig(u32, u32), // width, height
}

impl Display for PngReadError {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        use PngReadError::*;

        match self {
            DecodingError(err) => err.fmt(fmt),
            TooBig(w, h) => write!(fmt, "Image too big! ({} px wide, {} px tall)", w, h),
        }
    }
}

impl error::Error for PngReadError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        use PngReadError::*;

        match self {
            DecodingError(err) => Some(err),
            TooBig(..) => None,
        }
    }
}
