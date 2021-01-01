mod slices;
pub use slices::{parse_slices, Slice};

use std::ffi::OsStr;
use std::fs::File;
use std::io;

/// If the `OsStr` begins with an `@`,
pub fn process_leading_at(arg: &OsStr) -> Option<io::Result<File>> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        use std::str::from_utf8;

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
            .map(|_| File::open(OsStr::from_bytes(&bytes[1..]))) // THEN, we can use the rest of the string!
    }
    #[cfg(windows)]
    {
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
            .map(|_| File::open(OsString::from_wide(&units.collect::<Vec<u16>>())))
    }
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
