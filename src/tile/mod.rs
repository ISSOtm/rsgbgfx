mod block;
pub use block::Block;
use std::ops::Index;
use std::slice::SliceIndex;
mod palette;
pub use palette::Palettes;

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

    pub fn pixels(&'a self) -> PixelIterator<'a> {
        PixelIterator::new(self)
    }
}

impl<
        'a,
        I: SliceIndex<[&'a Color], Output = &'a Color>,
        J: SliceIndex<[[&'a Color; 8]], Output = [&'a Color; 8]>,
    > Index<(I, J)> for Tile<'a>
{
    type Output = &'a Color;

    fn index(&self, (x, y): (I, J)) -> &Self::Output {
        &self.pixels[y][x]
    }
}

pub struct PixelIterator<'a> {
    tile: &'a Tile<'a>,
    x: usize,
    y: usize,
}

impl<'a> PixelIterator<'a> {
    fn new(tile: &'a Tile) -> Self {
        Self { tile, x: 0, y: 0 }
    }
}

impl<'a> Iterator for PixelIterator<'a> {
    type Item = &'a Color;

    fn next(&mut self) -> Option<Self::Item> {
        if self.y == 8 {
            return None;
        }

        let color = &self.tile.pixels[self.y][self.x];
        self.x += 1;
        if self.x == 8 {
            self.x = 0;
            self.y += 1;
        }
        Some(color)
    }
}
