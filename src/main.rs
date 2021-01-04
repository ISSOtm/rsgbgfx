mod args;

mod img;
mod logic;
use logic::Params;
mod tile;
mod util;

use clap::{clap_app, crate_authors, crate_description, crate_version};
use std::env;
use std::io::{self, Write};

fn main() {
    let mut app = clap_app!(rsgbgfx =>
    (version: crate_version!())
    (author: crate_authors!())
    (about: crate_description!())
    (@arg no_discard: -D --"no-discard" "Disables discarding identical tiles (implies -V and -H)")
    (@arg no_horiz_flip: -H --"no-horizontal-flip" "Disables discarding tiles by flipping them horizontally")
    (@arg no_vert_flip: -V --"no-vertical-flip" "Disables discarding tiles by flipping them vertically")
    (@arg verbose: -v --verbose "Enable describing actions taken to stderr, repeat for more details")
    (@arg sprite: -s --sprite [color] #{0,1} "Enable OAM mode, and possibly force the background color") // TODO: "#n" to pick the nth color in the input palette, otherwise a color
    (@arg fuzzy: -f --fuzzy [threshold] #{0,1} "Treat colors similar enough as identical")
    (@arg base: -b --base [id] {util::parse_byte} default_value[0] "The base ID for tiles")
    (@arg bgp: -B --bgp [palette] {util::parse_byte} "This image's DMG palette")
    (@arg bpp: -d --depth [bpp] possible_value[1 2] default_value[2] "Number of bits per pixel")
    (@arg height: -h --height [height] default_value[1] "Height in tiles of a \"block\"")
    (@arg width: -w --width [width] default_value[1] "Width in tiles of a \"block\"")
    (@arg out_tiles: -o --"out-tiles" [path] "File name to output the tiles to")
    (@arg in_pal: -P --"in-palette" [palette] "Palette to use, or \"@path\" to read a file")
    (@arg out_pal: -p --"out-palette" [path] "File name to output the palette to")
    (@arg in_map: -T --"in-tilemap" [path] "File name to read a tilemap from")
    (@arg out_map: -t --"out-tilemap" [path] "File name to output the tilemap to")
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

    // clap has checked those args already
    let block_height = util::parse_byte(args.value_of("height").unwrap()).unwrap();
    let block_width = util::parse_byte(args.value_of("width").unwrap()).unwrap();

    let slice_ret = args.value_of_os("in_slices").map(|arg| {
        match args::process_leading_at(arg) {
            Some(Ok(file)) => args::parse_slices(file, block_width, block_height),
            Some(Err(err)) => {
                eprintln!("Error opening slices file: {}", err);
                std::process::exit(1);
            }
            None => args::parse_slices(arg.to_string_lossy().as_bytes(), block_width, block_height),
        }
        .unwrap_or_else(|err| {
            eprintln!("error parsing slices: {}", err);
            std::process::exit(1);
        })
    });
    let (slices, nb_blocks) = match slice_ret {
        Some((slices, nb_blocks)) => {
            if nb_blocks == 0 {
                todo!()
            } else {
                (Some(slices), nb_blocks)
            }
        }
        None => (None, 0),
    };

    /* Before <path> was made required, this is how its lack was handled...
    let path = match args.values_of("path") {
        Some(path) => path,
        None => {
            eprintln!("FATAL: No input file");
            let mut stderr = io::stderr();
            app.write_help(&mut stderr).unwrap();
            std::process::exit(1);
        }
    };
    */
    let params = Params {
        path: args.value_of_os("path").unwrap(),
        block_height,
        block_width,
        slices,
        nb_blocks,
    };

    // Remember: use `String::from_utf8_lossy` to display file names
    if let Err(err) = logic::process_file(params) {
        let mut stderr = io::stderr();
        writeln!(stderr, "error: {}", err).unwrap();
    }
}
