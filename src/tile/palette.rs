use crate::tile::Color;
use arrayvec::ArrayVec;
use std::convert::TryInto;

#[derive(Debug)]
pub struct Palette(ArrayVec<[[Color; 4]; 8]>);

impl Palette {
    pub fn new() -> Self {
        Self(ArrayVec::new())
    }

    pub fn nb_colors(&self) -> u8 {
        self.0.len().try_into().unwrap()
    }
}
