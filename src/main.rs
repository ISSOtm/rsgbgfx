mod img;

use img::{ImageReader, PngReader};
use std::fs::File;

fn main() {
    let f = File::open(
        // "/tmp/test.png",
        "/tmp/tiny.png",
    )
    .unwrap();
    let img = PngReader::new(f).unwrap().read_image().unwrap();
    println!("{:?}", img);
}
