use crate::args::Slice;
use crate::img::{self, ImageReader, PngReader};
use crate::tile::{Block, Palette, Tile};
use std::convert::TryFrom;
use std::error;
use std::fmt::{self, Display, Formatter};
use std::fs::File;
use std::io;
use std::path::{self, Path};

pub struct Params<'a, P: AsRef<Path> + ?Sized> {
    pub verbosity: u64,

    pub path: &'a P,
    pub tiles_path: Option<&'a P>,
    pub map_path: Option<&'a P>,
    pub attr_path: Option<&'a P>,

    pub block_height: u8,
    pub block_width: u8,

    pub slices: Option<Vec<Slice>>, // x, y (in pixels), w, h (in tiles)
    pub nb_blocks: usize,           // Hint to allocate the `Vec` up-front
    pub palette: Option<Palette>,

    pub no_discard: bool,
    pub no_horiz_flip: bool,
    pub no_vert_flip: bool,
    pub fuzziness: Fuzziness,
    pub base: u8,
    pub bgp: Option<u8>,
    pub bpp: u8,
}

#[derive(Debug)]
pub enum Fuzziness {
    /// Not specified on CLI
    /// Differs from `Threshold(0)` in that even identical colors won't be merged
    Strict,
    /// Specified on the CLI without an argument
    Closest,
    /// Specified on the CLI with an argument
    Threshold(u8),
}

pub fn process_file<P: AsRef<Path> + ?Sized>(params: Params<P>) -> Result<(), ProcessingError> {
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

        // TODO: Check starting and ending boundaries

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

    // Reduce colors depending on fuzziness
    // Do this before palette allocation to reduce tiles to 4 colors, so that discarding
    // opportunities may be used as palette allocation hints
    // If a palette was specified on the command-line, ensure also that all colors match it
    // TODO

    // Index tiles based on their 4 colors
    // This is used to check that all input tiles are 4 colors,
    // and to make it easier to permute colors in the tile for palette allocation
    // Additionally, if a palette was specified on the command-line, check that it contains all
    // of the image's colors; if not, report the `--fuzzy` radius that would make it work.

    todo!();
    Ok(())
}

#[derive(Debug)]
pub enum ProcessingError<'a> {
    HeightNotTiled(u32),
    WidthNotTiled(u32),
    HeightNotBlock(u32, u8),
    WidthNotBlock(u32, u8),
    Io(path::Display<'a>, io::Error),
    PngDecoding(png::DecodingError),
    PngReading(img::PngReadError),
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
            Io(name, err) => write!(fmt, "{}: {}", name, err),
            PngDecoding(err) => err.fmt(fmt),
            PngReading(err) => err.fmt(fmt),
        }
    }
}

impl error::Error for ProcessingError<'_> {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        use ProcessingError::*;

        match self {
            HeightNotTiled(..) | WidthNotTiled(..) => None,
            HeightNotBlock(..) | WidthNotBlock(..) => None,
            Io(_, err) => Some(err),
            PngDecoding(err) => Some(err),
            PngReading(err) => Some(err),
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
