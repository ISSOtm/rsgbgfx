use crate::args::Slice;
use crate::img::{self, ImageReader, PngReader};
use std::error;
use std::fmt::{self, Display, Formatter};
use std::fs::File;
use std::io;
use std::path::{self, Path};

pub struct Params<'a, P: AsRef<Path> + ?Sized> {
    pub path: &'a P,
    pub slices: Option<Vec<Slice>>, // x, y (in pixels), w, h (in tiles)
}

pub fn process_file<P: AsRef<Path> + ?Sized>(params: Params<P>) -> Result<(), ProcessingError> {
    let file = File::open(params.path)
        .map_err(|err| ProcessingError::Io(params.path.as_ref().display(), err))?;

    // TODO: Support other file formats?
    let img = PngReader::new(file)?.read_image()?;

    // Extract slices from image; use the whole image if none given
    for slice in match params.slices {
        Some(slices) => slices,
        None => {
            let (width, height) = (img.width(), img.height());
            if width % 8 != 0 {
                return Err(ProcessingError::BadWidth(width));
            }
            if height % 8 != 0 {
                return Err(ProcessingError::BadHeight(height));
            }
            vec![Slice {
                x: 0,
                y: 0,
                width: width / 8,
                height: height / 8,
            }]
        }
    } {
        todo!();
    }
    Ok(())
}

#[derive(Debug)]
pub enum ProcessingError<'a> {
    BadHeight(u32),
    BadWidth(u32),
    Io(path::Display<'a>, io::Error),
    PngDecoding(png::DecodingError),
    PngReading(img::PngReadError),
}

impl Display for ProcessingError<'_> {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        use ProcessingError::*;

        match self {
            BadHeight(height) => write!(fmt, "Height ({} px) cannot be divided by 8", height),
            BadWidth(width) => write!(fmt, "Width ({} px) cannot be divided by 8", width),
            Io(name, err) => write!(fmt, "{}: {}", name, err),
            PngDecoding(err) => err.fmt(fmt),
            PngReading(err) => err.fmt(fmt),
        }
    }
}

impl error::Error for ProcessingError<'_> {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        use ProcessingError::*;

        match self {
            BadHeight(_) | BadWidth(_) => None,
            Io(_, err) => Some(err),
            PngDecoding(err) => Some(err),
            PngReading(err) => Some(err),
        }
    }
}

impl From<png::DecodingError> for ProcessingError<'_> {
    fn from(err: png::DecodingError) -> Self {
        Self::PngDecoding(err)
    }
}

impl From<img::PngReadError> for ProcessingError<'_> {
    fn from(err: img::PngReadError) -> Self {
        Self::PngReading(err)
    }
}
