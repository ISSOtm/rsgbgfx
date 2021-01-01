use crate::img::{self, ImageReader, PngReader};
use std::error;
use std::fmt::{self, Display, Formatter};
use std::fs::File;
use std::io;
use std::path::Path;

pub fn process_file<P: AsRef<Path>>(path: P) -> Result<(), ProcessingError> {
    let file = File::open(path)?;
    // TODO: Support other file formats?
    let img = PngReader::new(file)?.read_image()?;
    Ok(())
}

#[derive(Debug)]
pub enum ProcessingError {
    Io(io::Error),
    PngDecoding(png::DecodingError),
    PngReading(img::PngReadError),
}

impl Display for ProcessingError {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        use ProcessingError::*;

        match self {
            Io(err) => err.fmt(fmt),
            PngDecoding(err) => err.fmt(fmt),
            PngReading(err) => err.fmt(fmt),
        }
    }
}

impl error::Error for ProcessingError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        use ProcessingError::*;

        match self {
            Io(err) => Some(err),
            PngDecoding(err) => Some(err),
            PngReading(err) => Some(err),
        }
    }
}

impl From<io::Error> for ProcessingError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<png::DecodingError> for ProcessingError {
    fn from(err: png::DecodingError) -> Self {
        Self::PngDecoding(err)
    }
}

impl From<img::PngReadError> for ProcessingError {
    fn from(err: img::PngReadError) -> Self {
        Self::PngReading(err)
    }
}
