use crate::img::Color;
use crate::tile::{Block, Tile};
use std::convert::{TryFrom, TryInto};
use std::io::{self, Write};

#[cfg(test)]
mod tests;

pub struct TileCollection {
    tiles: Vec<Vec<IndexedTile>>,
    base_tile_ids: Vec<u16>, // Base tile ID for each block
    attrs: Vec<u8>,          // Attribute for each block (vflip & hflip only)

    dedup: bool,
    horiz_flip: bool,
    vert_flip: bool,
}

pub const VFLIP_MASK: u8 = 0x40;
pub const HFLIP_MASK: u8 = 0x20;

impl TileCollection {
    pub fn new(dedup: bool, horiz_flip: bool, vert_flip: bool) -> Self {
        Self {
            tiles: Vec::new(),
            base_tile_ids: Vec::new(),
            attrs: Vec::new(),

            dedup,
            horiz_flip,
            vert_flip,
        }
    }

    pub fn add_block(&mut self, block: &Block, colors: &[Color]) {
        let tiles: Vec<_> = block
            .tiles()
            .iter()
            .map(|tile| IndexedTile::new(tile, colors))
            .collect();

        // See if we can find a redundant block
        let (blk_id, attr) = (|| {
            if self.dedup {
                for (i, block_tiles) in self.tiles.iter().enumerate() {
                    let redundancy = is_redundant(&tiles, block_tiles, block.width());

                    // If an allowed redundancy type is found, use that; otherwise, keep looping
                    let mask = if redundancy.identical {
                        0
                    } else if redundancy.vflip && self.vert_flip {
                        VFLIP_MASK
                    } else if redundancy.hflip && self.horiz_flip {
                        HFLIP_MASK
                    } else if redundancy.vhflip && self.vert_flip && self.horiz_flip {
                        VFLIP_MASK | HFLIP_MASK
                    } else {
                        // No redundancy found, keep iterating
                        continue;
                    };

                    return (i, mask);
                }
            }

            // Welp, no redundancy, so time to add ourselves
            let i = self.tiles.len();
            self.tiles.push(tiles);
            (i, 0)
        })();

        // Turn the block's ID into a tile ID, by multiplying with the block's amount of tiles
        self.base_tile_ids
            .push((blk_id * self.tiles[0].len()).try_into().unwrap());
        self.attrs.push(attr);
    }

    pub fn tiles(&self) -> impl Iterator<Item = &IndexedTile> + '_ {
        self.tiles.iter().flat_map(|block_tiles| block_tiles.iter())
    }

    pub fn base_tile_ids(&self) -> &[u16] {
        &self.base_tile_ids
    }

    pub fn attrs(&self) -> &[u8] {
        &self.attrs
    }
}

fn at<T>(array: &[T], x: usize, y: usize, width: usize) -> &T {
    &array[y * width + x]
}

fn is_redundant(lhs: &[IndexedTile], rhs: &[IndexedTile], width: usize) -> RedundancyTypes {
    assert_eq!(lhs.len(), rhs.len());
    assert_ne!(lhs.len(), 0);
    assert_eq!(lhs.len() % width, 0);
    let height = lhs.len() / width;

    let mut types = RedundancyTypes::new();
    for y in 0..(lhs.len() / width) {
        for x in 0..width {
            if at(lhs, x, y, width) != at(rhs, x, y, width) {
                types.identical = false;
            }
            if !at(lhs, x, height - 1 - y, width).is_vflip_of(at(rhs, x, y, width)) {
                types.vflip = false;
            }
            if !at(lhs, width - 1 - x, y, width).is_hflip_of(at(rhs, x, y, width)) {
                types.hflip = false;
            }
            if !at(lhs, width - 1 - x, height - 1 - y, width).is_vhflip_of(at(rhs, x, y, width)) {
                types.vhflip = false;
            }
        }
    }
    types
}

/// Lists **every** type of redundancy that's possible between two `IndexedTile`s.
#[derive(Debug)]
struct RedundancyTypes {
    pub identical: bool,
    pub vflip: bool,
    pub hflip: bool,
    /// When both are *needed*, not *possible*
    pub vhflip: bool,
}

impl RedundancyTypes {
    pub fn new() -> Self {
        Self {
            identical: true,
            vflip: true,
            hflip: true,
            vhflip: true,
        }
    }
}

/// A 2bpp, Game Boy-format tile.
// That is, 8 rows of 2 bytes each, with bitplane 0 first.
/// ("Bitplane N" means "One byte storing bit N of each pixel's index", the leftmost pixel being bit 7.)
#[derive(Debug, PartialEq, Eq)]
pub struct IndexedTile([u8; 16]);

impl IndexedTile {
    pub fn new(tile: &Tile, palette: &[Color]) -> Self {
        let mut bytes = [0; 16];

        for y in 0..8 {
            let (mut bp0, mut bp1) = (0, 0);
            for x in 0..8 {
                let index = palette
                    .iter()
                    .position(|elem| elem == tile[(x, y)])
                    .expect("Supplied an invalid palette for the tile");
                assert!(index < 4, "Got non-2bpp index {}", index);
                bp0 = bp0 << 1 | u8::try_from(index & 1).unwrap();
                bp1 = bp1 << 1 | u8::try_from(index >> 1).unwrap();
            }
            bytes[y * 2] = bp0;
            bytes[y * 2 + 1] = bp1;
        }

        Self(bytes)
    }

    /// Check that a given predicate holds for both bitplane of all rows.
    /// The arguments are the bitplane's index, and the index of the bitplane opposite vertically.
    /// So the inputs are: (0, 14), (1, 15), (2, 12), (3, 13), (4, 10), and so on
    fn check_all_rows<P: Fn(usize, usize) -> bool>(&self, predicate: P) -> bool {
        (0..8)
            .into_iter()
            .all(|y| predicate(y * 2, (7 - y) * 2) && predicate(y * 2 + 1, (7 - y) * 2 + 1))
    }

    fn is_vflip_of(&self, other: &IndexedTile) -> bool {
        self.check_all_rows(|y, ry| self.0[y] == other.0[ry])
    }

    fn is_hflip_of(&self, other: &IndexedTile) -> bool {
        self.check_all_rows(|y, _| self.0[y] == (other.0[y]).reverse_bits())
    }

    fn is_vhflip_of(&self, other: &IndexedTile) -> bool {
        self.check_all_rows(|y, ry| self.0[y] == (other.0[ry]).reverse_bits())
    }

    pub fn write_to(&self, output: &mut impl Write, bpp: u8) -> io::Result<()> {
        if bpp == 1 {
            for i in 0..8 {
                assert_eq!(self.0[i * 2 + 1], 0);
                output.write_all(&self.0[i * 2..=i * 2])?;
            }
            Ok(())
        } else {
            output.write_all(&self.0)
        }
    }
}
