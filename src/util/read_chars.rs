use std::convert::TryFrom;
use std::error;
use std::fmt::{self, Display, Formatter};
use std::io::{self, Bytes, Read};

#[derive(Debug)]
pub struct CharReader<R: Read> {
    bytes: Bytes<R>,
}

impl<R: Read> CharReader<R> {
    pub fn new(bytes: Bytes<R>) -> Self {
        Self { bytes }
    }
}

impl<R: Read> Iterator for CharReader<R> {
    type Item = Result<char, CharReaderError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.bytes.next().map(|first| {
            let first = first?;
            let mut bytes = [first, 0, 0, 0];
            let n = usize::try_from(first.leading_ones()).unwrap();
            if n >= 2 && n <= 4 {
                // `n` indicates the total amount of bytes, but the first one has already been read
                for b in bytes.iter_mut().skip(1).take(n - 1) {
                    *b = match self.bytes.next() {
                        None => Err(CharReaderError::BadUtf8),
                        Some(Err(err)) => Err(CharReaderError::Io(err)),
                        Some(Ok(b)) => Ok(b),
                    }?;
                }
            } else if n != 0 {
                return Err(CharReaderError::BadUtf8);
            }
            Ok(std::str::from_utf8(&bytes)
                .map_err(|_| CharReaderError::BadUtf8)?
                .chars()
                .next()
                .unwrap())
        })
    }
}

#[derive(Debug)]
pub enum CharReaderError {
    BadUtf8,
    Io(io::Error),
}

impl From<io::Error> for CharReaderError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl Display for CharReaderError {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        use CharReaderError::*;

        match self {
            BadUtf8 => write!(fmt, "Invalid UTF-8"),
            Io(err) => write!(fmt, "I/O error: {}", err),
        }
    }
}

impl error::Error for CharReaderError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        use CharReaderError::*;

        match self {
            BadUtf8 => None,
            Io(err) => Some(err),
        }
    }
}
