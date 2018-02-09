extern crate image;
extern crate image_processor;
extern crate terminal_graphics;
extern crate terminal_size;
extern crate clap;

use std::fs::File;
use std::path::Path;

use image::GenericImage;
use image::SubImage;
use image::Pixel;

use terminal_graphics::Display;
use terminal_graphics::Colour;

use terminal_size::{Width, Height, terminal_size};

use image_processor::{merge_pixels, get_image, pixelate_image};

use clap::{Arg, App, SubCommand};

fn main() {
    let matches = App::new("Termage")
                    .version("0.1")
                    .about("Display any image in the terminal with Termage!")
                    .author("Calum")
                    .arg(Arg::with_name("image")
                            .short("i")
                            .long("image")
                            .value_name("FILE")
                            .help("Input image filepath")
                            .required(true)
                            .takes_value(true))
                    .get_matches();

    let image_filepath = matches.value_of("image").unwrap();

    let size = terminal_size();
    if let Some((Width(w), Height(h))) = size {
        display_image(image_filepath, w as u32, h as u32);
    } else {
        println!("Error: Unable to get terminal dimensions.");
    }
}

fn display_image(image_filepath: &str, width: u32, height: u32) {
    let mut img = get_image(image_filepath);
    let mut screen = Display::new(width, height);
    screen.clear();

    let img_out = pixelate_image(&mut img, width, height);

    for (x, y, pixel) in img_out.enumerate_pixels() {
        let rgb = pixel.to_rgb().data;
        let colour = Colour::from_rgb(rgb[0], rgb[1], rgb[2]);

        screen.set_pixel(x as isize, y as isize, ' ', Colour::White, colour);
    }

    screen.print();
    println!("");

    // Save the image
    let ref mut fout = File::create(&Path::new("test.png")).unwrap();

    // We must indicate the image’s color type and what format to save as
    let _ = image::ImageRgb8(img_out).save(fout, image::PNG);
}
