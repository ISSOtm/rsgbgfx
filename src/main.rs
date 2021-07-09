mod args;
mod img;
mod logic;
use logic::Params;
mod tile;
mod util;

use clap::{clap_app, crate_authors, crate_description, crate_version};
use std::convert::TryFrom;
use std::env;
use std::fs::File;
use std::io;
use std::io::Write;
use std::process;

fn main() {
    // TODO: Color curves (convert colors emitted to the palette files into what would produce them if displayed by a certain console)
    // TODO: A flag to respect the input PNG's palette IDs
    let mut app = clap_app!(rsgbgfx =>
    (version: crate_version!())
    (author: crate_authors!())
    (about: crate_description!())
    (@arg dedup: -D --"deduplicate" "Enable discarding identical tiles (implies -V and -H)")
    (@arg horiz_flip: -H --"horizontal-flip" "Enable discarding tiles by flipping them horizontally")
    (@arg vert_flip: -V --"vertical-flip" "Enable discarding tiles by flipping them vertically")
    (@arg verbose: -v --verbose ... "Enable describing actions taken to stderr, repeat for more details")
    (@arg sprite: -s --sprite [color] #{0,1} "Enable OAM mode, and possibly force the background color") // TODO: "#n" to pick the nth color in the input palette, otherwise a color
    (@arg base: -b --base [id] {util::parse_byte} default_value[0] "The base ID for tiles")
    (@arg bgp: -B --bgp [palette] {util::parse_byte} "This image's DMG palette")
    (@arg bpp: -d --depth [bpp] possible_value[1 2] default_value[2] "Number of bits per pixel")
    (@arg height: -h --height [height] default_value[1] "Height in tiles of a \"block\"")
    (@arg width: -w --width [width] default_value[1] "Width in tiles of a \"block\"")
    (@arg out_tiles: -o --"out-tiles" [path] "File name to output the tiles to")
    (@arg in_pal: -P --"in-palette" [palette] "Palette to use, or \"@path\" to read a RGBA8888 file")
    (@arg out_pal: -p --"out-palette" [path] "File name to output the native palettes to")
    (@arg out_pal_rgba8888: --"out-palette-rgba8888" [path] "File name to output the RGBA8888 palettes to")
    (@arg out_pal_map: --"out-palmap" [path] "File name to output the palette map to")
    (@arg out_map: -t --"out-tilemap" [path] "File name to output the tilemap to")
    (@arg out_himap: --"out-himap" [path] "File name to output the \"high\" tilemap to")
    (@arg out_attr: -a --"out-attrmap" [path] "File name to output the GBC attribute map to")
    (@arg in_slices: -S --slices [slices] "Slices to use, or \"@path\" to read a file")
    (@arg path: * "Path to the input image")
    );

    // By default, `clap` prints to stdout, but we want stderr, so handle printing ourselves
    // We need `app` to outlive the argument parsing for printing!
    let args = match app.try_get_matches_from_mut(env::args_os()) {
        Ok(args) => args,
        Err(clap::Error {
            kind: clap::ErrorKind::DisplayHelp,
            ..
        }) => {
            let mut stderr = io::stderr();
            app.write_help(&mut stderr).unwrap();
            std::process::exit(0);
        }
        Err(clap::Error {
            kind: clap::ErrorKind::DisplayVersion,
            ..
        }) => {
            eprint!("{}", app.render_version()); // `render_version` ends with a newline
            std::process::exit(0);
        }
        Err(e) => e.exit(),
    };

    let dedup = args.is_present("dedup");
    let horiz_flip = args.is_present("horiz_flip");
    let vert_flip = args.is_present("vert_flip");
    let verbosity = args.occurrences_of("verbose");
    // All the `unwrap`s are because clap checked them already (required argument,
    // `util::parse_byte` already run as a validator, etc.)
    let base = util::parse_byte(args.value_of("base").unwrap()).unwrap();
    let bgp = args
        .value_of("bgp")
        .map(|string| util::parse_byte(string).unwrap());
    let bpp = args.value_of("bpp").unwrap().parse().unwrap();
    let block_height = util::parse_byte(args.value_of("height").unwrap()).unwrap();
    let block_width = util::parse_byte(args.value_of("width").unwrap()).unwrap();

    let slice_ret = args
        .value_of_os("in_slices")
        .map(|arg| match args::read_leading_at(arg) {
            Some(Ok(vec)) => {
                args::parse_slices(&*vec, block_width, block_height).unwrap_or_else(|err| {
                    eprintln!("Error parsing slices: {}", err);
                    std::process::exit(1);
                })
            }
            Some(Err(err)) => {
                eprintln!("Error opening slices file: {}", err);
                std::process::exit(1)
            }
            None => args::parse_slices(arg.to_string_lossy().as_bytes(), block_width, block_height)
                .unwrap_or_else(|err| {
                    eprintln!("Error parsing slices: {}", err);
                    std::process::exit(1);
                }),
        });
    let (slices, nb_blocks) = match slice_ret {
        Some((slices, nb_blocks)) => {
            // The slice parser should treat `nb_blocks == 0` as an error
            debug_assert_ne!(nb_blocks, 0);
            (Some(slices), nb_blocks)
        }
        None => (None, 0),
    };

    // If a palette was supplied on the CLI, either read the "@file", or process it directly
    let palette = args
        .value_of_os("in_pal")
        .map(|arg| match args::process_leading_at(arg) {
            Some(Ok(file)) => args::palette::read(file).unwrap_or_else(|err| {
                eprintln!("Error processing palette file: {}", err);
                std::process::exit(1);
            }),
            Some(Err(err)) => {
                eprintln!("Error opening palette file: {}", err);
                std::process::exit(1);
            }
            None => args::palette::parse(arg.to_string_lossy().chars()).unwrap_or_else(|err| {
                eprintln!("Error reading palette: {}", err);
                std::process::exit(1);
            }),
        });
    // TODO: if both fuzziness and palette are given, warn if there is ambiguity

    let params = Params {
        verbosity,

        path: args.value_of_os("path").unwrap(),

        block_height,
        block_width,

        slices,
        nb_blocks,
        palette,

        dedup,
        horiz_flip,
        vert_flip,
        base,
        bgp,
        bpp,
    };

    // Now, process all of that!

    // Remember: use `String::from_utf8_lossy` to display file names
    let (palettes, pal_map, tile_data) = logic::process_file(params).unwrap_or_else(|err| {
        eprintln!("error: {}", err);
        process::exit(1);
    });

    let block_size = u16::from(block_height) * u16::from(block_width);

    // Output time!
    // TODO: use `BufWriter`s

    if let Some(path) = args.value_of_os("out_pal") {
        match File::open(path) {
            Err(err) => eprintln!("Error opening palette output file: {}", err),
            Ok(mut file) => (|| {
                for palette in &palettes {
                    for color in palette {
                        file.write_all(&color.to_rgb555().to_le_bytes())?;
                    }
                }
                Ok(())
            })()
            .unwrap_or_else(|err: io::Error| eprintln!("Error writing palette: {}", err)),
        }
    }

    if let Some(path) = args.value_of_os("out_pal_rgba8888") {
        match File::open(path) {
            Err(err) => eprintln!("Error opening RGBA8888 palette output file: {}", err),
            Ok(mut file) => (|| {
                for palette in &palettes {
                    for color in palette {
                        file.write_all(&color.rgba())?;
                    }
                }
                Ok(())
            })()
            .unwrap_or_else(|err: io::Error| eprintln!("Error writing RGBA8888 palette: {}", err)),
        }
    }

    if let Some(path) = args.value_of_os("out_tiles") {
        match File::open(path) {
            Err(err) => eprintln!("Error opening tile output file: {}", err),
            Ok(mut file) => (|| {
                for tile in tile_data.tiles() {
                    tile.write_to(&mut file, bpp)?;
                }
                Ok(())
            })()
            .unwrap_or_else(|err: io::Error| eprintln!("Error writing tiles: {}", err)),
        }
    }

    let pal_map_path = args.value_of_os("out_palmap");
    if let Some(path) = pal_map_path {
        match File::open(path) {
            Err(err) => eprintln!("Error opening palette map output file: {}", err),
            Ok(mut file) => (|| {
                for entry in &pal_map {
                    file.write_all(&entry.to_le_bytes())?;
                }
                Ok(())
            })()
            .unwrap_or_else(|err: io::Error| eprintln!("Error writing palette map: {}", err)),
        }
    }

    let output_tilemap = |index, file: &mut File| {
        for base_id in tile_data.base_tile_ids() {
            for ofs in 0..(block_size) {
                // Only write the bottom byte
                file.write_all(&(base_id + ofs).to_le_bytes()[index..=index])?;
            }
        }
        Ok(())
    };
    if let Some(path) = args.value_of_os("out_map") {
        match File::open(path) {
            Err(err) => eprintln!("Error opening tilemap output file: {}", err),
            Ok(mut file) => output_tilemap(0, &mut file)
                .unwrap_or_else(|err: io::Error| eprintln!("Error writing tilemap: {}", err)),
        }
    }
    if let Some(path) = args.value_of_os("out_himap") {
        match File::open(path) {
            Err(err) => eprintln!("Error opening high tilemap output file: {}", err),
            Ok(mut file) => output_tilemap(1, &mut file)
                .unwrap_or_else(|err: io::Error| eprintln!("Error writing high tilemap: {}", err)),
        }
    }

    if let Some(path) = args.value_of_os("out_attrmap") {
        // TODO: warn if more than 8 palettes and palette map is not demanded
        if palettes.len() > 8 && !args.is_present("out-palmap") {
            eprintln!(
                "Warning: {} palettes generated, but palette map not requested",
                palettes.len()
            );
        }

        match File::open(path) {
            Err(err) => eprintln!("Error opening attrmap output file: {}", err),
            Ok(mut file) => (|| {
                assert_eq!(tile_data.attrs().len(), pal_map.len());

                for (attr, pal) in tile_data.attrs().iter().zip(pal_map.iter()) {
                    for _ in 0..block_size {
                        let pal_id = if args.is_present("out-palmap") {
                            0
                        } else {
                            u8::try_from(pal & 7).unwrap()
                        };

                        file.write_all(&[*attr | pal_id])?;
                    }
                }
                Ok(())
            })()
            .unwrap_or_else(|err: io::Error| eprintln!("Error writing attrmap: {}", err)),
        }
    }
}
