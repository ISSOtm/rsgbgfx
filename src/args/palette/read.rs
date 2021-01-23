use crate::tile::Palette;
use std::error;
use std::fmt::{self, Display, Formatter};
use std::fs::File;
use std::io::{self, Read};

/// Either a PNG files (as identified by the first 8 magic bytes), or a raw RGB555 palette file
pub fn read(mut file: File) -> Result<Palette, ReadError> {
    // Read the first 8 bytes, and see if they match the PNG magic bytes
    let mut first8 = [0; 8];
    let result = file.read_exact(&mut first8);
    // http://www.libpng.org/pub/png/spec/iso/index-object.html#5PNG-file-signature
    static PNG_MAGIC: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

    match result {
        Ok(()) if first8 == PNG_MAGIC => {
            // Magic bytes matched!
            let pal = Palette::new();

            todo!()
        }
        // Early EOF may just be a small palette file
        Err(err) if err.kind() != io::ErrorKind::UnexpectedEof => {
            // I/O error
            Err(err.into())
        }
        Ok(()) | Err(_) => {
            // Raw RGB555 colors
            let pal = Palette::new();

            todo!()
        }
    }
}

#[derive(Debug)]
pub enum ReadError {
    Io(io::Error),
    Variant2,
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
            Io(err) => err.fmt(fmt),
            Variant2 => todo!(),
        }
    }
}

impl error::Error for ReadError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        use ReadError::*;

        match self {
            Io(err) => Some(err),
            Variant2 => None,
        }
    }
}
