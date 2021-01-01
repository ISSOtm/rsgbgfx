use std::io::Read;

// Everything's public because it's plain ol' data
#[derive(Debug)]
pub struct Slice {
    // In pixels
    pub x: u32,
    pub y: u32,
    // In tiles
    pub width: u32,
    pub height: u32,
}

pub fn parse_slices(input: &mut dyn Read) -> Vec<Slice> {
    unimplemented!()
}
