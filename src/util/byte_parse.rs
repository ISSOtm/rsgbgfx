use std::convert::TryFrom;
use std::error;
use std::fmt::{self, Display, Formatter};

pub fn parse_byte(string: &str) -> Result<u8, ByteParseError> {
    let mut chars = string.trim().chars().peekable();

    let negative = match chars.peek() {
        Some('+') => {
            chars.next();
            false
        }
        Some('-') => {
            chars.next();
            true
        }
        _ => false,
    };

    let (radix, got_digit) = match chars.peek() {
        Some('0') => {
            chars.next();
            if let Some('X') | Some('x') = chars.peek() {
                chars.next();
                (16, false)
            } else {
                (8, true) // Accept "0"...
            }
        }
        Some('$') => {
            chars.next();
            (16, false)
        }
        Some('%') => {
            chars.next();
            (2, false)
        }
        Some(_) => (10, true),
        None => (10, false),
    };

    if !got_digit && chars.peek() == None {
        return Err(ByteParseError::Empty);
    }

    // Iterate on the remaining chars
    let mut val: u8 = 0;
    for c in chars {
        let digit = match c.to_digit(radix.into()) {
            Some(digit) => u8::try_from(digit).unwrap(),
            None => return Err(ByteParseError::BadChar(c, radix)),
        };
        // Multiply by the radix and add the digit, returning OutOfRange if either overflows
        val = val
            .checked_mul(radix)
            .and_then(|val| val.checked_add(digit))
            .ok_or(ByteParseError::OutOfRange)?;
    }

    if !negative {
        Ok(val)
    } else {
        Ok(i8::try_from(-i16::from(val)).map_err(|_| ByteParseError::OutOfRange)? as u8)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ByteParseError {
    BadChar(char, u8),
    Empty,
    OutOfRange,
}

impl Display for ByteParseError {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        use ByteParseError::*;

        match self {
            BadChar(c, radix) => write!(fmt, "Invalid character '{}' for base {}", c, radix),
            Empty => write!(fmt, "Empty number"),
            OutOfRange => write!(fmt, "Number not in range [-128; 255]"),
        }
    }
}

impl error::Error for ByteParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: test whitespace trimming

    #[test]
    fn parse_decimal() {
        for i in -128..=255 {
            assert_eq!(parse_byte(format!("{}", i).as_str()).unwrap(), i as u8);
        }
    }

    #[test]
    fn parse_hex() {
        for i in 0..=255 {
            assert_eq!(parse_byte(format!("0x{:X}", i).as_str()).unwrap(), i as u8);
            assert_eq!(parse_byte(format!("0X{:X}", i).as_str()).unwrap(), i as u8);
            assert_eq!(parse_byte(format!("0x{:x}", i).as_str()).unwrap(), i as u8);
            assert_eq!(parse_byte(format!("0X{:x}", i).as_str()).unwrap(), i as u8);
        }
        for i in -128..=0 {
            assert_eq!(
                parse_byte(format!("-0x{:X}", -i).as_str()).unwrap(),
                i as u8
            );
            assert_eq!(
                parse_byte(format!("-0X{:X}", -i).as_str()).unwrap(),
                i as u8
            );
            assert_eq!(
                parse_byte(format!("-0x{:x}", -i).as_str()).unwrap(),
                i as u8
            );
            assert_eq!(
                parse_byte(format!("-0X{:x}", -i).as_str()).unwrap(),
                i as u8
            );
        }
    }

    #[test]
    fn parse_dollar() {
        for i in 0..=255 {
            assert_eq!(parse_byte(format!("${:x}", i).as_str()).unwrap(), i as u8);
        }
        for i in -128..=0 {
            assert_eq!(parse_byte(format!("-${:x}", -i).as_str()).unwrap(), i as u8);
        }
    }

    #[test]
    fn parse_bin() {
        for i in 0..=255 {
            assert_eq!(parse_byte(format!("%{:b}", i).as_str()).unwrap(), i as u8);
        }
        for i in -128..=0 {
            assert_eq!(parse_byte(format!("-%{:b}", -i).as_str()).unwrap(), i as u8);
        }
    }

    #[test]
    fn parse_oct() {
        for i in 0..=255 {
            assert_eq!(parse_byte(format!("0{:o}", i).as_str()).unwrap(), i as u8);
        }
        for i in -128..=0 {
            assert_eq!(parse_byte(format!("-0{:o}", -i).as_str()).unwrap(), i as u8);
        }
    }

    #[test]
    fn out_of_range() {
        assert_eq!(parse_byte("-129").unwrap_err(), ByteParseError::OutOfRange);
        assert_eq!(parse_byte("256").unwrap_err(), ByteParseError::OutOfRange);
        assert_eq!(parse_byte("-0x81").unwrap_err(), ByteParseError::OutOfRange);
        assert_eq!(parse_byte("0x100").unwrap_err(), ByteParseError::OutOfRange);
        assert_eq!(parse_byte("-0201").unwrap_err(), ByteParseError::OutOfRange);
        assert_eq!(parse_byte("0400").unwrap_err(), ByteParseError::OutOfRange);
    }

    #[test]
    fn empty() {
        assert_eq!(parse_byte("").unwrap_err(), ByteParseError::Empty);
        assert_eq!(parse_byte("+").unwrap_err(), ByteParseError::Empty);
        assert_eq!(parse_byte("-").unwrap_err(), ByteParseError::Empty);
        assert_eq!(parse_byte("0x").unwrap_err(), ByteParseError::Empty);
        assert_eq!(parse_byte("+0x").unwrap_err(), ByteParseError::Empty);
        assert_eq!(parse_byte("-0x").unwrap_err(), ByteParseError::Empty);
        assert_eq!(parse_byte("$").unwrap_err(), ByteParseError::Empty);
        assert_eq!(parse_byte("+$").unwrap_err(), ByteParseError::Empty);
        assert_eq!(parse_byte("-$").unwrap_err(), ByteParseError::Empty);
    }

    #[test]
    fn bad_char() {
        assert_eq!(
            parse_byte("2a").unwrap_err(),
            ByteParseError::BadChar('a', 10)
        );
        assert_eq!(
            parse_byte("$2G").unwrap_err(),
            ByteParseError::BadChar('G', 16)
        );
        assert_eq!(
            parse_byte("08").unwrap_err(),
            ByteParseError::BadChar('8', 8)
        );
        assert_eq!(
            parse_byte("++").unwrap_err(),
            ByteParseError::BadChar('+', 10)
        );
        assert_eq!(
            parse_byte("+-").unwrap_err(),
            ByteParseError::BadChar('-', 10)
        );
        assert_eq!(
            parse_byte("-+").unwrap_err(),
            ByteParseError::BadChar('+', 10)
        );
        assert_eq!(
            parse_byte("--").unwrap_err(),
            ByteParseError::BadChar('-', 10)
        );
    }
}
