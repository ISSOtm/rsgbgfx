use crate::tile::Palettes;
use std::error;
use std::fmt::{self, Display, Formatter};

use std::io::{self};

/// Parses a textual palette spec
pub fn parse<I: Iterator<Item = char>>(string: I) -> Result<Palettes, ParseError> {
    let pal = Palettes::new();
    unimplemented!()
}

#[derive(Debug)]
pub enum ParseError {
    Io(io::Error),
    Variant2,
}

impl From<io::Error> for ParseError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl Display for ParseError {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        use ParseError::*;

        match self {
            Io(err) => err.fmt(fmt),
            Variant2 => todo!(),
        }
    }
}

impl error::Error for ParseError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        use ParseError::*;

        match self {
            Io(err) => Some(err),
            Variant2 => None,
        }
    }
}
