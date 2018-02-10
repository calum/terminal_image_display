extern crate image;
extern crate image_processor;
extern crate terminal_graphics;
extern crate terminal_size;
extern crate clap;

use image::Pixel;

use terminal_graphics::Display;
use terminal_graphics::Colour;

use terminal_size::{Width, Height, terminal_size};

use image_processor::{get_image, pixelate_image};

use clap::{Arg, App};

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

    // get the image dimensions
    let (img_width, img_height) = img.dimensions();
    let ratio = (img_width as f32)/(img_height as f32);

    let mut display_height = height;
    let mut display_width = width;

    // scale the image to the correct dimensions
    if ratio > 1.00 {
        display_height = ((display_width as f32)/(ratio * 2.0)).floor() as u32;
    } else {
        display_width = 2 * ((display_height as f32) * ratio).floor() as u32;
    }

    // scale the image up if it is too small to fit the dimensions of the terminal
    let mut scale = 1.00;
    if (display_width as f32)*scale > (img_width as f32) {
        scale = (img_width as f32)/(width as f32);
    }
    if (display_height as f32)*scale > (img_height as f32) {
        scale = (img_height as f32)/(height as f32);
    }

    // scale the width and height
    display_width = ((display_width as f32)*scale).floor() as u32;
    display_height = ((display_height as f32)*scale).floor() as u32;

    // scale the display to fit the terminal:
    scale = 1.0;
    if (display_width as f32) * scale > (width as f32) {
        scale = (width as f32) / (display_width as f32);
    }
    if (display_height as f32) * scale > (height as f32) {
        scale = (height as f32) / (display_height as f32);
    }

    // scale the width and height
    display_width = ((display_width as f32)*scale).floor() as u32;
    display_height = ((display_height as f32)*scale).floor() as u32;

    let mut screen = Display::new(display_width, display_height);
    screen.clear();

    let img_out = pixelate_image(&mut img, display_width, display_height);

    for (x, y, pixel) in img_out.enumerate_pixels() {
        let rgb = pixel.to_rgb().data;
        let colour = Colour::from_rgb(rgb[0], rgb[1], rgb[2]);

        screen.set_pixel(x as isize, y as isize, ' ', Colour::White, colour);
    }

    screen.print();

    // Save the image
    //let ref mut fout = File::create(&Path::new("test.png")).unwrap();

    // We must indicate the image’s color type and what format to save as
    //let _ = image::ImageRgb8(img_out).save(fout, image::PNG);
}
