use crate::util::{CharReader, CharReaderError};
use std::convert::TryFrom;
use std::error;
use std::fmt::{self, Display, Formatter};
use std::io::{self, Read};
use std::iter::Peekable;

// Everything's public because it's plain ol' data
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Slice {
    // In pixels
    pub x: u32,
    pub y: u32,
    // In tiles
    pub width: u32,
    pub height: u32,
}

impl Display for Slice {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        write!(
            fmt,
            "(x: {}, y: {}, width: {}, height: {})",
            self.x, self.y, self.width, self.height
        )
    }
}

pub fn parse_slices<T: Read>(
    input: T,
    block_width: u8,
    block_height: u8,
) -> Result<(Vec<Slice>, usize), ParseError> {
    let (mut slices, mut nb_blocks) = (vec![], 0);
    let mut chars = CharReader::new(input.bytes()).peekable();

    skip_whitespace(&mut chars, true)?; // Skip initial whitespace
    loop {
        match chars.peek() {
            Some(Ok('#')) => {
                // Consume characters until EOL (or EOF)
                if loop {
                    match chars.next().transpose()? {
                        Some('\n') => break false,
                        None => break true,
                        _ => (),
                    }
                } {
                    // If we reached EOF, exit the outer loop as well
                    break;
                }
                // Skip initial whitespace again
                skip_whitespace(&mut chars, true)?;
            }
            Some(Ok(_)) => {
                // Expected format: `<x> <y> <w> <h>`, may be separated by any amount of whitespace
                // (at least 1), may be decimal, octal (0), hexadecimal (both 0x and $)
                let x = try_parse_number(&mut chars, "x")?;
                skip_whitespace(&mut chars, false)?;
                let y = try_parse_number(&mut chars, "y")?;
                skip_whitespace(&mut chars, false)?;
                let width = try_parse_number(&mut chars, "width")?;
                skip_whitespace(&mut chars, false)?;
                let height = try_parse_number(&mut chars, "height")?;
                skip_whitespace(&mut chars, false)?;

                // Check that the slice's dimensions are multiples of the block's
                if width % u32::from(block_width) != 0 {
                    return Err(ParseError::NonIntWidth(width, block_width));
                }
                if height % u32::from(block_height) != 0 {
                    return Err(ParseError::NonIntHeight(height, block_height));
                }

                // Append the slice to the `Vec`
                slices.push(Slice {
                    x,
                    y,
                    width,
                    height,
                });
                nb_blocks += usize::try_from(
                    (width / u32::from(block_width)) * (height / u32::from(block_height)),
                )
                .map_err(|_| ParseError::TooManyBlocks)?;

                // Skip trailing whitespace
                skip_whitespace(&mut chars, false)?;
                match chars.next().transpose()? {
                    // Comma and newlines, aka "slice separators": read any whitespace, and get ready for next statement
                    Some(',') | Some('\n') => skip_whitespace(&mut chars, true)?,
                    // Comment: continue, will be discarded at the top of the loop
                    Some('#') => (),
                    Some(c) => return Err(ParseError::IllegalChar(c)),
                    None => break,
                }
            }
            None => break,
            Some(Err(_)) => {
                return Err(chars
                    .next()
                    .transpose()
                    .expect_err("Peekable magically un-errored itself!?")
                    .into())
            }
        }
    }

    if nb_blocks == 0 {
        // 0 blocks means we didn't scan any meaningful entries
        Err(ParseError::Empty)
    } else {
        Ok((slices, nb_blocks))
    }
}

fn skip_whitespace<R: Read>(
    input: &mut Peekable<CharReader<R>>,
    accept_newlines: bool,
) -> Result<(), CharReaderError> {
    loop {
        match input.peek() {
            Some(Err(_)) => {
                return Err(input
                    .next()
                    .transpose()
                    .expect_err("Peekable magically un-errored itself!?"))
            }
            Some(Ok(c)) if c.is_whitespace() && (accept_newlines || *c != '\n') => {
                input
                    .next()
                    .transpose()
                    .expect("Peekable magically errored itself!?");
            }
            _ => return Ok(()),
        }
    }
}

fn try_parse_number<R: Read>(
    input: &mut Peekable<CharReader<R>>,
    name: &'static str,
) -> Result<u32, ParseError> {
    let (radix, mut number): (_, Option<u32>) = match input.peek() {
        // We'll let errors be handled below
        Some(Err(_)) => (10, None),
        None => return Err(ParseError::UnexpectedEof),
        Some(Ok('$')) => {
            input
                .next()
                .transpose()
                .expect("Peekable magically errored itself!?");
            (16, None)
        }
        Some(Ok('0')) => {
            input
                .next()
                .transpose()
                .expect("Peekable magically errored itself!?");
            // This might either be octal, or hexadecimal
            if let Some(Ok('x')) | Some(Ok('X')) = input.peek() {
                input
                    .next()
                    .transpose()
                    .expect("Peekable magically errored itself!?");
                (16, None)
            } else {
                (8, Some(0))
            }
        }
        Some(Ok(_)) => (10, None),
    };

    let err = loop {
        match input.peek() {
            Some(Err(_)) => {
                return Err(input
                    .next()
                    .transpose()
                    .expect_err("Peekable magically un-errored itself!?")
                    .into());
            }
            Some(Ok(c)) if c.is_digit(radix) => {
                number = Some(
                    number
                        .unwrap_or(0)
                        .checked_mul(radix)
                        .and_then(|n| n.checked_add(c.to_digit(radix).unwrap()))
                        .ok_or(ParseError::TooLarge(name))?,
                );
            }
            Some(Ok(c)) => break ParseError::IllegalChar(*c),
            None => break ParseError::UnexpectedEof,
        }
        input
            .next()
            .transpose()
            .expect("Peekable magically errored itself!?")
            .unwrap();
    };

    number.ok_or(err)
}

#[derive(Debug)]
pub enum ParseError {
    BadUtf8,
    Empty,
    IllegalChar(char),
    Io(io::Error),
    NonIntHeight(u32, u8),
    NonIntWidth(u32, u8),
    TooLarge(&'static str),
    TooManyBlocks,
    UnexpectedEof,
}

impl Display for ParseError {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        use ParseError::*;

        match self {
            BadUtf8 => write!(fmt, "Invalid UTF-8 sequence"),
            Empty => write!(fmt, "No slices specified"),
            IllegalChar(c) => write!(fmt, "Illegal character '{}'", c.escape_debug()),
            Io(err) => write!(fmt, "I/O error: {}", err),
            NonIntHeight(slice, block) => write!(
                fmt,
                "Slice's height ({} tiles) is not a multiple of block's ({} tiles)",
                slice, block
            ),
            NonIntWidth(slice, block) => write!(
                fmt,
                "Slice's width ({} tiles) is not a multiple of block's ({} tiles)",
                slice, block
            ),
            TooLarge(which) => write!(fmt, "{} too large", which),
            TooManyBlocks => write!(fmt, "Too many blocks, try splitting this image"),
            UnexpectedEof => write!(fmt, "Unexpected end of input"),
        }
    }
}

impl From<CharReaderError> for ParseError {
    fn from(err: CharReaderError) -> Self {
        match err {
            CharReaderError::BadUtf8 => ParseError::BadUtf8,
            CharReaderError::Io(err) => ParseError::Io(err),
        }
    }
}

impl error::Error for ParseError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        use ParseError::*;

        match self {
            BadUtf8 => None,
            Empty => None,
            IllegalChar(..) => None,
            Io(err) => Some(err),
            NonIntHeight(..) | NonIntWidth(..) => None,
            TooLarge(..) => None,
            TooManyBlocks => None,
            UnexpectedEof => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ParseError::*;

    macro_rules! parse_test_inner {
        ($input:expr => Ok([ $( $x:expr, $y:expr, $w:expr, $h:expr );* ], $nb_blocks:pat)) => {
            let ret = $input;
            if let Ok((slices, $nb_blocks)) = ret {
                assert_eq!(slices, [ $( Slice { x:$x, y:$y, width:$w, height:$h } ),* ]);
            } else {
                panic!("{:?}", ret);
            }
        };
        ($input:expr => Err($err:pat)) => {
            let ret = $input;
            if let Err($err) = ret {
            } else {
                panic!("{:?}", ret);
            }
        };
    }

    macro_rules! parse_slices {
        (($str:tt)) => {
            parse_slices!(($str, 0, 0))
        };
        (([ $( $byte:expr ),* ], $w:expr, $h:expr)) => {
            parse_slices!((&[ $( $byte ),* ][..], $w, $h))
        };
        (($str:literal, $w:expr, $h:expr)) => {
            parse_slices!(($str.as_bytes(), $w, $h))
        };
        (($str:expr, $w:expr, $h:expr)) => {
            parse_slices($str, $w, $h)
        };
    }

    macro_rules! parse_test {
        // Use this when the other params don't matter (e.g. when not expecting to process any entries)
        ($( #[$attrs:meta] )* $name:ident, { $( $args:tt ),+ } => $type:ident $expected:tt) => {
            // Since the input must be a string (for non-ASCII chars), we can't directly pass in
            // byte literals.
            #[allow(clippy::string_lit_as_bytes)]
            #[test]
            $( #[$attrs] )*
            fn $name() {
                $( parse_test_inner!{parse_slices!($args) => $type $expected} )+
            }
        };
    }

    parse_test! {empty, {("")} => Err(Empty)}
    parse_test! {partial, {("0")} => Err(UnexpectedEof)}
    parse_test! {unk_char, {("ù")} => Err(IllegalChar('ù'))}

    parse_test! {just_comment, {("# Hello yes I am comment")} => Err(Empty)}
    parse_test! {empty_comment, {("#")} => Err(Empty)}
    parse_test! {comment_after_comment, {("# Hello yes I am comment
# Incredible, me too!
#")} => Err(Empty)}
    parse_test! {hash_me, {("####")} => Err(Empty)}
    parse_test! {space_before_comment, {(" \t  # Comment!")} => Err(Empty)}

    // http://www.unicode.org/versions/Unicode13.0.0/ch03.pdf#G27506
    parse_test! {utf8, {("# Un café, ça vous va ?")} => Err(Empty)}
    parse_test! {bad_utf8_first, {
        ([0x80]),
        ([0xc1]), ([0xc1, 0x80]),
        ([0xf5]), ([0xf5, 0x80, 0x80])
    } => Err(BadUtf8)}
    // Not enough bytes / not enough *continuation* bytes
    parse_test! {bad_utf8_len, {
        ([0xc2]), ([0xc2, 0x00]),
        ([0xe1]), ([0xe1, 0x80]), ([0xe1, 0x80, 0x00]),
        ([0xf3]), ([0xf3, 0x80, 0x80]), ([0xf3, 0x80, 0x80, 0x00])
    } => Err(BadUtf8)}
    // Length is otherwise valid, the second byte is in the 0x80-0xBF range, but not valid in this context
    parse_test! {bad_utf8_2nd, {
        ([0xe0, 0x9f]),
        ([0xed, 0xa0, 0x80]),
        ([0xf0, 0x8f, 0x80, 0x80]),
        ([0xf4, 0x90, 0x80, 0x80])
    } => Err(BadUtf8)}

    parse_test! {#[should_panic] zero_blk_width, {("0 0 1 1", 0, 1)} => Err(..)}
    parse_test! {#[should_panic] zero_blk_height, {("0 0 1 1", 1, 0)} => Err(..)}

    parse_test! {slice, {("0 0 1 1", 1, 1)} => Ok([0,0,1,1], 1)}
    parse_test! {slices_comma, {("0 0 1 1,8 0 1 1", 1, 1)} => Ok([0,0,1,1; 8,0,1,1], 2)}
    parse_test! {slices_comma_spaces, {("0 0 1 1,\t \t\t8 0 1 1", 1, 1)} => Ok([0,0,1,1; 8,0,1,1], 2)}
    parse_test! {slices_newline, {("0 0 1 1\n 8 0 1 1", 1, 1)} => Ok([0,0,1,1; 8,0,1,1], 2)}
    parse_test! {trailing_comma, {("0 0 1 1,\n8 0 1 1", 1, 1)} => Ok([0,0,1,1; 8,0,1,1], 2)}
    parse_test! {trailing_comma_comment, {("0 0 1 1, # Blep.", 1, 1)} => Ok([0,0,1,1], 1)}
    parse_test! {trailing_comma_eof, {("0 0 1 1,", 1, 1)} => Ok([0,0,1,1], 1)}
    parse_test! {trailing_comma_eof_spaces, {("0 0 1 1, \t  \t", 1, 1)} => Ok([0,0,1,1], 1)}

    parse_test! {nb_blks, {("0 0 2 2 , 8 0 1 2", 1, 2)} => Ok([0,0,2,2; 8,0,1,2], 3)}
    parse_test! {bad_width, {("0 0 2 2\t, 8 0 1 2", 2, 2)} => Err(NonIntWidth(1, 2))}
    parse_test! {bad_height, {("0 0 2 2\t, 8 0 2 1", 2, 2)} => Err(NonIntHeight(1, 2))}
}
