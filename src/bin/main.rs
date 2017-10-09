extern crate image;
extern crate image_processor;
extern crate terminal_graphics;

use std::fs::File;
use std::path::Path;

use image::GenericImage;
use image::SubImage;
use image::Pixel;

use terminal_graphics::Display;
use terminal_graphics::Colour;

use image_processor::{merge_pixels, get_image, pixelate_image};

fn main() {
    let mut img = get_image("ferris.png");

    let mut screen = Display::new(200,100);
    screen.clear();

    let img_out = pixelate_image(&mut img, 200, 100);

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
