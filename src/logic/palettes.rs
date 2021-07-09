use super::ProcessingError;
use crate::img::Color;
use crate::tile::Block;
use std::convert::TryInto;

pub fn pack_palettes<'a, 'b>(
    blocks: &'a [Block],
    pal_map: &'a mut Vec<u16>,
    bpp: u8,
) -> Result<Vec<[Color; 4]>, ProcessingError<'b>> {
    let mut block_palettes = vec![Some(Vec::with_capacity(1 << bpp)); blocks.len()];

    // First, determine the colors used by each block
    for (i, block) in blocks.iter().enumerate() {
        let colors = block_palettes[i].as_mut().unwrap(); // We only have `Some`s at this point

        for tile in block.tiles() {
            for pixel in tile.pixels() {
                if !colors.contains(&pixel) {
                    if colors.len() == 1 << bpp {
                        return Err(ProcessingError::TooManyColors(
                            block.x(),
                            block.y(),
                            block.width(),
                            block.height(),
                            bpp,
                        ));
                    }

                    colors.push(pixel);
                }
            }
        }
    }

    // From the "requests", generate the palettes
    // This amounts to solving the bin packing problem, yes, but we can usually simplify it a bit
    let mut palettes = PaletteCollection::new();

    // First step: any palettes that are fully-sized, will be kept as-is, obviously
    for i in 0..blocks.len() {
        if let Some(pal) = &block_palettes[i] {
            if pal.len() == 1 << bpp {
                // Replace the palette in the Vec with a None, and retrieve it
                let mut palette = [None];
                block_palettes[i..=i].swap_with_slice(&mut palette);

                // Add the palette to the vector
                pal_map[i] = palettes.insert(&palette[0].unwrap()).unwrap();
            }
        }
    }

    // Now, prune subsets of already-allocated palettes
    for i in 0..block_palettes.len() {
        if let Some(pal) = &block_palettes[i] {
            //
        }
    }

    // Now, if any palettes remain, allocate them
    if block_palettes.iter().any(|pal| pal.is_some()) {
        todo!();
    }

    // TODO: remember to add magenta as padding!

    Ok(palettes.gen_palettes(&Color::new((0xFF, 0x00, 0xFF, 0xFF), None)))
}

struct PaletteCollection(Vec<[Option<Color>; 4]>);

impl PaletteCollection {
    pub fn new() -> Self {
        // There are typically 8 palettes at most
        Self(Vec::with_capacity(8))
    }

    pub fn gen_palettes(self, filler: &Color) -> Vec<[Color; 4]> {
        unimplemented!()
    }

    pub fn insert(&mut self, palette: &[Color]) -> Option<u16> {
        self.find(palette).or_else(|| {
            // Ensure that we don't hit more than 65536 palettes
            if self.0.len() == 65536 {
                return None;
            }

            // If the palette wasn't found, try to find one it could share colors with
            // TODO: this may be suboptimal, but the problem is NP-complete...
            // TODO

            // None found? Alright then, add a new palette slot
            self.0.resize_with(self.0.len() + 1, Default::default);
            let new_pal = self.0.last().unwrap();
            for i in 0..palette.len() {
                todo!();
            }
            Some(self.0.len().try_into().unwrap())
        })
    }

    pub fn find(&self, palette: &[Color]) -> Option<u16> {
        self.0.iter().enumerate().find_map(|(i, pal)| {
            // See if the target contains all of our colors
            palette
                .iter()
                .all(|color| {
                    pal.iter()
                        .any(|item| item.as_ref().map_or(false, |c| c == color))
                })
                // If so, return the ID
                .then(|| i.try_into().unwrap())
        })
    }
}
