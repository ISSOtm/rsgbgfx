mod args;

mod img;
mod logic;
use logic::{Fuzziness, Params};
mod tile;
mod util;

use clap::{clap_app, crate_authors, crate_description, crate_version};
use std::env;
use std::io::{self, Read, Write};

fn main() {
    // TODO: Color curves (convert colors emitted to the palette files into what would produce them if displayed by a certain console)
    let mut app = clap_app!(rsgbgfx =>
    (version: crate_version!())
    (author: crate_authors!())
    (about: crate_description!())
    (@arg no_discard: -D --"no-discard" "Disable discarding identical tiles (implies -V and -H)")
    (@arg no_horiz_flip: -H --"no-horizontal-flip" "Disable discarding tiles by flipping them horizontally")
    (@arg no_vert_flip: -V --"no-vertical-flip" "Disable discarding tiles by flipping them vertically")
    (@arg verbose: -v --verbose ... "Enable describing actions taken to stderr, repeat for more details")
    (@arg sprite: -s --sprite [color] #{0,1} "Enable OAM mode, and possibly force the background color") // TODO: "#n" to pick the nth color in the input palette, otherwise a color
    (@arg fuzzy: -f --fuzzy [threshold] #{0,1} {util::parse_byte} "Treat colors similar enough as identical")
    (@arg base: -b --base [id] {util::parse_byte} default_value[0] "The base ID for tiles")
    (@arg bgp: -B --bgp [palette] {util::parse_byte} "This image's DMG palette")
    (@arg bpp: -d --depth [bpp] possible_value[1 2] default_value[2] "Number of bits per pixel")
    (@arg height: -h --height [height] default_value[1] "Height in tiles of a \"block\"")
    (@arg width: -w --width [width] default_value[1] "Width in tiles of a \"block\"")
    (@arg out_tiles: -o --"out-tiles" [path] "File name to output the tiles to")
    (@arg in_pal: -P --"in-palette" [palette] "Palette to use, or \"@path\" to read a file")
    (@arg out_pal: -p --"out-palette" [path] "File name to output the palette to")
    (@arg out_map: -t --"out-tilemap" [path] "File name to output the tilemap to")
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

    // TODO: sprite, out_tiles, out_pal, in_map, out_map, in_attr, out_attr
    let no_discard = args.is_present("no_discard");
    let no_horiz_flip = args.is_present("no_horiz_flip");
    let no_vert_flip = args.is_present("no_vert_flip");
    let verbosity = args.occurrences_of("verbose");
    // All the `unwrap`s are because clap checked them already (required argument,
    // `util::parse_byte` already run as a validator, etc.)
    let fuzziness = args
        .values_of("fuzzy")
        .map_or(Fuzziness::Strict, |mut values| {
            values.next().map_or(Fuzziness::Closest, |string| {
                Fuzziness::Threshold(util::parse_byte(string).unwrap())
            })
        });
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

    let tiles_path = args.value_of_os("out_tiles");
    let map_path = args.value_of_os("out_map");
    let attr_path = args.value_of_os("out_attr");

    // If out_map and/or out_attr is set, there may only be one slice
    if (map_path.is_some() || attr_path.is_some())
        && slices.as_ref().map_or(false, |slices| slices.len() != 1)
    {
        eprintln!("Error: A tilemap or attrmap cannot be produced from more than one slice");
        std::process::exit(1);
    }

    let params = Params {
        verbosity,

        path: args.value_of_os("path").unwrap(),
        tiles_path,
        map_path,
        attr_path,

        block_height,
        block_width,

        slices,
        nb_blocks,
        palette,

        no_discard,
        no_horiz_flip,
        no_vert_flip,
        fuzziness,
        base,
        bgp,
        bpp,
    };

    // Remember: use `String::from_utf8_lossy` to display file names
    if let Err(err) = logic::process_file(params) {
        let mut stderr = io::stderr();
        writeln!(stderr, "error: {}", err).unwrap();
    }
}
