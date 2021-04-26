use crate::img::Color;
use crate::tile::Palette;
use png::{Decoder, DecodingError};
use std::error;
use std::fmt::{self, Display, Formatter};
use std::fs::File;
use std::io::{self, BufReader, Read};

/// Reads a palette spec from the file it's given.
/// Either a PNG file (as identified by the first 8 magic bytes), or a raw RGB555 palette file.
pub fn read(file: File) -> Result<Palette, ReadError> {
    let mut file = BufReader::new(file);

    // Read the first 8 bytes, and see if they match the PNG magic bytes
    let mut first8 = [0; 8];
    let result = file.read_exact(&mut first8);
    let mut data = first8.chain(file);
    // http://www.libpng.org/pub/png/spec/iso/index-object.html#5PNG-file-signature
    static PNG_MAGIC: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

    match result {
        Ok(()) if first8 == PNG_MAGIC => {
            // Magic bytes matched!
            let pal = Palette::new();
            let png = Decoder::new(data);

            todo!()
        }

        // Early EOF may just be a small palette file
        Err(err) if err.kind() != io::ErrorKind::UnexpectedEof => {
            // I/O error
            Err(err.into())
        }

        Ok(()) | Err(_) => {
            // Raw RGB555 colors
            let mut pal = Palette::new();
            let mut color = [0; 2];

            loop {
                match data.read_exact(&mut color) {
                    Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => break,
                    Err(err) => return Err(err.into()),
                    Ok(()) => {
                        let color = u16::from(color[0]) + u16::from(color[1]) * 256;

                        if pal.push(Color::from_rgb555(color, None)).is_err() {
                            return Err(ReadError::TooManyColors);
                        }
                    }
                }
            }
            Ok(pal)
        }
    }
}

#[derive(Debug)]
pub enum ReadError {
    BadPng(DecodingError),
    Io(io::Error),
    TooManyColors,
}

impl From<DecodingError> for ReadError {
    fn from(err: DecodingError) -> Self {
        Self::BadPng(err)
    }
}

impl From<io::Error> for ReadError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl Display for ReadError {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        use ReadError::*;

        match self {
            BadPng(err) => write!(
                fmt,
                "PNG file detected, but got an error while decoding: {}",
                err
            ),
            Io(err) => err.fmt(fmt),
            TooManyColors => write!(fmt, ""),
        }
    }
}

impl error::Error for ReadError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        use ReadError::*;

        match self {
            BadPng(err) => Some(err),
            Io(err) => Some(err),
            TooManyColors => None,
        }
    }
}
