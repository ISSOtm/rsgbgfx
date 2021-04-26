mod slices;
pub use slices::{parse_slices, Slice};
pub mod palette;

use std::ffi::OsStr;
use std::fs::{self, File};
use std::io;

#[cfg(unix)]
fn has_leading_at(arg: &OsStr) -> Option<&OsStr> {
    use std::os::unix::ffi::OsStrExt;

    let bytes = arg.as_bytes();
    debug_assert_eq!('@'.len_utf8(), 1);
    // The argument begins with a '@' if...
    bytes
        .get(0) // ...there is a first byte...
        .filter(|c| {
            let mut expected = [0]; // Length checked by debug assertion above
            '@'.encode_utf8(&mut expected);
            **c == expected[0]
        }) // ...and it's an 'at' sign.
        .map(|_| OsStr::from_bytes(&bytes[1..])) // THEN, we can use the rest of the string!
}

#[cfg(windows)]
fn has_leading_at(arg: &OsStr) -> Option<OsString> {
    use std::ffi::OsString;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};

    let mut units = arg.encode_wide();
    debug_assert_eq!('@'.len_utf16(), 1);
    // The argument begins with a '@' if...
    units
        .next() // ... there is a first unit...
        .filter(|unit| {
            let mut expected = [0]; // Length checked by debug assertion above
            '@'.encode_utf16(&mut expected);
            *unit == expected[0]
        }) // ...and it's an 'at' sign.
        // THEN, we can use the rest of the string!
        .map(|_| OsString::from_wide(&units.collect::<Vec<u16>>()))
}

/// If the `OsStr` begins with an `@`, treat the rest as a path, and try opening that file.
/// Otherwise, return `None`.
pub fn process_leading_at(arg: &OsStr) -> Option<io::Result<File>> {
    has_leading_at(arg).map(File::open)
}

pub fn read_leading_at(arg: &OsStr) -> Option<io::Result<Vec<u8>>> {
    has_leading_at(arg).map(fs::read)
}

/*
 * This used to be tested while the function returned strings, and had a different name.
 * All tests passed under Unix.
 * However, Windows requiring the creation of a new owned `OsString` required returning a `File`,
 * so the tests can't be done anymore.

#[cfg(test)]
mod tests {
    use super::*;

    fn str_to_osstr(arg: &str) -> &OsStr {
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;

            OsStr::from_bytes(arg.as_bytes())
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::{OsStrExt, OsStringExt};

            unimplemented!()
        }
    }

    #[test]
    fn leading_at() {
        assert_eq!(
            remove_leading_at(str_to_osstr("@test")),
            Some(str_to_osstr("test"))
        )
    }

    #[test]
    fn no_leading_at() {
        assert_eq!(remove_leading_at(str_to_osstr("test")), None)
    }

    #[test]
    fn empty() {
        assert_eq!(remove_leading_at(str_to_osstr("")), None)
    }

    #[test]
    fn empty_at() {
        assert_eq!(remove_leading_at(str_to_osstr("@")), Some(str_to_osstr("")))
    }
}
*/
