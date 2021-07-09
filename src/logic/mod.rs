use crate::args::Slice;
use crate::img::{self, Color, ImageReader, PngReader};
use crate::tile::{Block, Palettes, Tile};
use std::convert::TryFrom;
use std::convert::TryInto;
use std::error;
use std::fmt::{self, Display, Formatter};
use std::fs::File;
use std::io;
use std::ops::Deref;
use std::path::{self, Path};

mod palettes;
mod tiles;
pub use tiles::TileCollection;

pub struct Params<'a, P: AsRef<Path> + ?Sized> {
    pub verbosity: u64,

    pub path: &'a P,

    pub block_height: u8,
    pub block_width: u8,

    pub slices: Option<Vec<Slice>>, // x, y (in pixels), w, h (in tiles)
    pub nb_blocks: usize,           // Hint to allocate the `Vec` up-front
    pub palette: Option<Palettes>,

    pub dedup: bool,
    pub horiz_flip: bool,
    pub vert_flip: bool,
    pub base: u8,
    pub bgp: Option<u8>,
    pub bpp: u8,
}

pub fn process_file<P: AsRef<Path> + ?Sized>(
    params: Params<P>,
) -> Result<(Vec<[Color; 4]>, Vec<u16>, TileCollection), ProcessingError> {
    let (blk_width, blk_height) = (
        u32::from(params.block_width),
        u32::from(params.block_height),
    );
    let file = File::open(params.path)
        .map_err(|err| ProcessingError::Io(params.path.as_ref().display(), err))?;

    // TODO: Support other file formats?
    let img = PngReader::new(file)?.read_image()?;

    // If no slices were given, use the whole image
    let (width, height) = (img.width(), img.height());
    let whole_image = [Slice {
        x: 0,
        y: 0,
        width: width / 8,
        height: height / 8,
    }];
    let (slices, nb_blocks) = match params.slices.as_ref() {
        Some(slices) => (slices.as_slice().iter(), params.nb_blocks),
        None => {
            if width % 8 != 0 {
                return Err(ProcessingError::WidthNotTiled(width));
            }
            if height % 8 != 0 {
                return Err(ProcessingError::HeightNotTiled(height));
            }
            if width % (blk_width) != 0 {
                return Err(ProcessingError::WidthNotBlock(width, params.block_width));
            }
            if height % (blk_height) != 0 {
                return Err(ProcessingError::HeightNotBlock(height, params.block_height));
            }
            (
                whole_image.iter(),
                ((width / blk_width) * (height / blk_height)) as usize,
            )
        }
    };

    // Extract tiles from the image
    let mut blocks = Vec::with_capacity(nb_blocks);

    for slice in slices {
        // These should have been checked at slice creation
        debug_assert_ne!(slice.height, 0);
        debug_assert_ne!(slice.width, 0);
        debug_assert_eq!(slice.height % blk_height, 0);
        debug_assert_eq!(slice.width % blk_width, 0);

        // Check starting and ending boundaries
        if img.width() - slice.x < slice.width * 8 || img.height() - slice.y < slice.height * 8 {
            return Err(ProcessingError::OobSlice(slice.clone()));
        }

        let base = blocks.len(); // Base index of blocks about to be added
        let height_blk = slice.height / blk_height; // Slice's height in blocks
        let width_blk = slice.width / blk_width; // Slice's width in blocks
        let nb_blocks = (height_blk * width_blk) as usize; // Amount of blocks to add
        let mut coords = (0..width_blk).flat_map(|x| {
            (0..height_blk)
                .map(move |y| (slice.x + x * 8 * blk_width, slice.y + y * 8 * blk_height))
        });
        blocks.resize_with(base + nb_blocks, || {
            Block::new(params.block_width.into(), coords.next().unwrap())
        });

        // Generate tiles vertically first, as 8x16 mode requires contiguous vertical tile IDs
        for ofs_x in 0..slice.width {
            for ofs_y in 0..slice.height {
                let tile = Tile::from_image(&img, slice.x + ofs_x * 8, slice.y + ofs_y * 8);
                let idx = usize::try_from(ofs_x).unwrap() * usize::try_from(height_blk).unwrap()
                    + usize::try_from(ofs_y).unwrap();
                assert!(
                    idx < nb_blocks,
                    "Index {} is greater than expected {} blocks",
                    idx,
                    nb_blocks
                );
                blocks[base + idx].add_tile(tile);
            }
        }
    }

    // Generate the palette map, which maps one palette per block
    // If a palette spec was given on the command line, ensure that tiles match it
    // Otherwise, generate palettes from the colors used by tiles, checking that there are only 4 per tile

    let mut pal_map = vec![0; nb_blocks]; // One entry per block, top to bottom, left to right

    let palettes = if let Some(pal) = params.palette {
        // Check that the palette's size matches the bpp setting
        for (i, palette) in pal.deref().iter().enumerate() {
            if palette.len() > 1 << params.bpp {
                return Err(ProcessingError::BppMismatch(i, palette.len(), params.bpp));
            }
        }

        for (i, block) in blocks.iter().enumerate() {
            // Find a suitable palette for the whole block
            let mut is_candidate = vec![true; pal.nb_palettes().into()];

            for tile in block.tiles() {
                for pixel in tile.pixels() {
                    for i in 0..usize::from(pal.nb_palettes()) {
                        // Don't perform a costly check if the palette has already been eliminated
                        // TODO: if the color has already been seen, no need to look it up again
                        if is_candidate[i] && !pal[i].contains(pixel) {
                            is_candidate[i] = false;
                        }
                    }
                }
            }

            // Since the palette is already given on the CLI, we don't need to try to optimize: just pick one
            if let Some((index, _)) = is_candidate.iter().enumerate().find(|(_, &yes)| yes) {
                pal_map[i] = index.try_into().unwrap();
            } else {
                return Err(ProcessingError::NoPaletteFor(
                    block.x(),
                    block.y(),
                    block.width(),
                    block.height(),
                ));
            }
        }

        pal.colors()
    } else {
        palettes::pack_palettes(&blocks, &mut pal_map, params.bpp)?
    };

    // Generate tile data, keeping them grouped by blocks

    let mut tile_data = TileCollection::new(params.dedup, params.horiz_flip, params.vert_flip);

    for (block, pal_id) in blocks.iter().zip(&pal_map) {
        tile_data.add_block(block, &palettes[usize::from(*pal_id)]);
    }

    // TODO: try rotating colors in the palettes to improve flipping optimization

    Ok((palettes, pal_map, tile_data))
}

#[derive(Debug)]
pub enum ProcessingError<'a> {
    HeightNotTiled(u32),
    WidthNotTiled(u32),
    HeightNotBlock(u32, u8),
    WidthNotBlock(u32, u8),
    BppMismatch(usize, usize, u8),
    Io(path::Display<'a>, io::Error),
    NoPaletteFor(u32, u32, usize, usize),
    OobSlice(Slice),
    PngDecoding(png::DecodingError),
    PngReading(img::PngReadError),
    TooManyColors(u32, u32, usize, usize, u8),
}

impl Display for ProcessingError<'_> {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        use ProcessingError::*;

        match self {
            HeightNotTiled(height) => {
                write!(fmt, "Image height ({} px) cannot be divided by 8", height)
            }
            WidthNotTiled(width) => {
                write!(fmt, "Image width ({} px) cannot be divided by 8", width)
            }
            HeightNotBlock(height, block) => write!(
                fmt,
                "Image height ({} tiles) cannot be divided by block's ({} tiles)",
                height, block
            ),
            WidthNotBlock(width, block) => write!(
                fmt,
                "Image width ({} tiles) cannot be divided by block's ({} tiles)",
                width, block
            ),
            BppMismatch(id, cnt, bpp) => write!(
                fmt,
                "Palette #{} contains {} colors, but {}bpp palettes can only contain up to {}",
                id,
                cnt,
                bpp,
                1 << bpp
            ),
            Io(name, err) => write!(fmt, "{}: {}", name, err),
            // Report the block's size in pixels
            NoPaletteFor(x, y, w, h) => write!(
                fmt,
                "No palette for block (x: {}, y: {}, width: {}, height: {})",
                x,
                y,
                w * 8,
                h * 8
            ),
            OobSlice(slice) => write!(fmt, "Slice {} is not within the image's bounds", slice),
            PngDecoding(err) => err.fmt(fmt),
            PngReading(err) => err.fmt(fmt),
            TooManyColors(x, y, w, h, bpp) => write!(
                fmt,
                "Block (x: {}, y: {}, width: {}, height: {}) contains more than {} colors",
                x,
                y,
                w * 8,
                h * 8,
                1 << bpp
            ),
        }
    }
}

impl error::Error for ProcessingError<'_> {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        use ProcessingError::*;

        match self {
            HeightNotTiled(..) | WidthNotTiled(..) => None,
            HeightNotBlock(..) | WidthNotBlock(..) => None,
            BppMismatch(..) => None,
            Io(_, err) => Some(err),
            NoPaletteFor(..) => None,
            OobSlice(..) => None,
            PngDecoding(err) => Some(err),
            PngReading(err) => Some(err),
            TooManyColors(..) => None,
        }
    }
}

impl From<png::DecodingError> for ProcessingError<'_> {
    fn from(err: png::DecodingError) -> Self {
        Self::PngDecoding(err)
    }
}

impl From<img::PngReadError> for ProcessingError<'_> {
    fn from(err: img::PngReadError) -> Self {
        Self::PngReading(err)
    }
}
