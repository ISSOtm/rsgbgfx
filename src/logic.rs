use crate::args::Slice;
use crate::img::{self, ImageReader, PngReader};
use crate::tile::{Block, Tile};
use std::error;
use std::fmt::{self, Display, Formatter};
use std::fs::File;
use std::io;
use std::path::{self, Path};

pub struct Params<'a, P: AsRef<Path> + ?Sized> {
    pub path: &'a P,

    pub block_height: u8,
    pub block_width: u8,

    pub slices: Option<Vec<Slice>>, // x, y (in pixels), w, h (in tiles)
    pub nb_blocks: usize,           // Hint to allocate the `Vec` up-front
}

pub fn process_file<P: AsRef<Path> + ?Sized>(params: Params<P>) -> Result<(), ProcessingError> {
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
            if width % (params.block_width as u32) != 0 {
                return Err(ProcessingError::WidthNotBlock(width, params.block_width));
            }
            if height % (params.block_height as u32) != 0 {
                return Err(ProcessingError::HeightNotBlock(height, params.block_height));
            }
            (
                whole_image.iter(),
                ((width / params.block_width as u32) * (height / params.block_height as u32))
                    as usize,
            )
        }
    };

    // Extract tiles from the image; use the whole image if no slices are given
    let mut blocks = Vec::with_capacity(nb_blocks);

    for slice in slices {
        // These should have been checked at slice creation
        debug_assert_ne!(slice.height, 0);
        debug_assert_ne!(slice.width, 0);
        debug_assert_eq!(slice.height % params.block_height as u32, 0);
        debug_assert_eq!(slice.width % params.block_width as u32, 0);

        // TODO: Check starting and ending boundaries

        let base = blocks.len(); // Base index of blocks about to be added
        let height_blk = (slice.height / params.block_height as u32) as usize; // Slice's height in blocks
        let nb_blocks = height_blk * (slice.width / params.block_width as u32) as usize; // Amount of blocks to add
        blocks.resize_with(base + nb_blocks, || Block::new(params.block_width.into()));

        // Generate tiles vertically first, as 8x16 mode requires contiguous vertical tile IDs
        for ofs_x in 0..slice.width {
            for ofs_y in 0..slice.height {
                let tile = Tile::from_image(&img, slice.x + ofs_x * 8, slice.y + ofs_y * 8);
                let idx = ofs_x as usize * height_blk + ofs_y as usize;
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

    for block in &blocks {
        debug_assert_eq!(block.height(), params.block_height.into());
        println!("Blk{{...}}");
    }
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
