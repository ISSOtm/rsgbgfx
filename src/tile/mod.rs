mod block;
pub use block::Block;
mod palette;
pub use palette::Palette;

use crate::img::{Color, Image};
use arrayvec::ArrayVec;

#[derive(Debug)]
pub struct Tile<'a> {
    pixels: [[&'a Color; 8]; 8],
    // Coordinates (for reporting location in errors)
    x: u32,
    y: u32,
}

impl<'a> Tile<'a> {
    pub fn from_image(img: &'a Image, x: u32, y: u32) -> Self {
        Self {
            pixels: (y..(y + 8))
                .into_iter()
                .map(|y| {
                    (x..(x + 8))
                        .into_iter()
                        .map(|x| &img[(x, y)])
                        .collect::<ArrayVec<_>>()
                        .into_inner()
                        .unwrap()
                })
                .collect::<ArrayVec<_>>()
                .into_inner()
                .unwrap(),
            x,
            y,
        }
    }
}
