extern crate image;
extern crate terminal_graphics;

use terminal_graphics::Display;
use terminal_graphics::Colour;

use std::path::Path;
use std::fs::File;
use std::{thread, time};

use image::{GenericImage, Pixel, RgbImage, gif, Frames, ImageDecoder, Frame};
use image::{ConvertBuffer, ImageBuffer};

pub fn pixelate_image(img: &mut RgbImage, width: u32, height: u32) -> RgbImage {
    // Create a new ImgBuf
    let mut imgbuf = image::ImageBuffer::new(width, height);

    closest_match(&mut imgbuf, img);

    imgbuf
}

/// Opens the file and reads a gif animated image.
/// Returns the Frames for the gif.
pub fn get_gif(filename: &str) -> Frames {
    let mut f = File::open(filename).expect("File not found");

    let mut decoder = gif::Decoder::new(f);

    decoder.into_frames().expect("error decoding gif")
}

pub fn get_image(filename: &str) -> RgbImage {
    let img = image::open(&Path::new(filename)).unwrap();

    img.to_rgb()
}

/// Downsizes the image using the closest match algorithm
pub fn closest_match(imgbuf: &mut RgbImage, img: &mut RgbImage) {
    // The dimensions method returns the images width and height
    let (width, height) = img.dimensions();
    let (out_width, out_height) = imgbuf.dimensions();

    // set the variables needed to average out the pixels
    let scale_x = (width as f32)/(out_width as f32);
    let scale_y = (height as f32)/(out_height as f32);

    // Iterate over the coordinates and pixels of the image
    for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
        let x_f32 = x as f32;
        let y_f32 = y as f32;

        let closest_pixel_x = (x_f32 * scale_x).floor() as u32;
        let closest_pixel_y = (y_f32 * scale_y).floor() as u32;

        let closest_pixel = img.get_pixel(closest_pixel_x, closest_pixel_y);

        // Create an 8bit pixel of type RGB
        *pixel = closest_pixel.clone();
    }
}

/// Averages all rgb values of a group of pixels. Causes dimming.
pub fn merge_pixels(imgbuf: &mut RgbImage, img: &mut RgbImage) {
    // The dimensions method returns the images width and height
    let (width, height) = img.dimensions();
    let (out_width, out_height) = imgbuf.dimensions();

    // set the variables needed to average out the pixels
    let scale_x = width/out_width;
    let scale_y = height/(out_height);
    let num_pixels = (scale_y*scale_x) as f32;

    // Iterate over the coordinates and pixels of the image
    for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {

        // create a large pixel which will equal the average of all the
        // pixels in the sub image
        let mut big_pixel_rgb: [f32; 3] = [0.0, 0.0, 0.0];

        // create the sub image of size equal to the size of the big pixel
        let big_pixel = img.sub_image(x*scale_x, y*scale_y, scale_x, scale_y);

        // average the rgb values
        for (_, _, pixel) in big_pixel.pixels() {
            let rgb_values = pixel.to_rgb().data;

            big_pixel_rgb[0] += rgb_values[0] as f32;
            big_pixel_rgb[1] += rgb_values[1] as f32;
            big_pixel_rgb[2] += rgb_values[2] as f32;
        }
        big_pixel_rgb[0] *= 1.0/num_pixels;
        big_pixel_rgb[1] *= 1.0/num_pixels;
        big_pixel_rgb[2] *= 1.0/num_pixels;

        // Create an 8bit pixel of type RGB
        *pixel = image::Rgb([big_pixel_rgb[0] as u8, big_pixel_rgb[1] as u8, big_pixel_rgb[2] as u8]);
    }
}

fn render_image(mut image: RgbImage, width: u32, height: u32) {
    // get the image dimensions
    let (img_width, img_height) = image.dimensions();
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

    let img_out = pixelate_image(&mut image, display_width, display_height);

    for (x, y, pixel) in img_out.enumerate_pixels() {
        let rgb = pixel.to_rgb().data;
        let colour = Colour::from_rgb(rgb[0], rgb[1], rgb[2]);

        screen.set_pixel(x as isize, y as isize, ' ', Colour::White, colour);
    }

    screen.print();
    println!("");
}

pub fn display_image(image_filepath: &str, width: u32, height: u32) {
    let mut img = get_image(image_filepath);

    render_image(img, width, height);
}

pub fn display_gif(gif_filepath: &str, width: u32, height: u32) {
    // get the original gif
    let mut frames = get_gif(gif_filepath);

    // create the new reduced gif by shrinking each frame to fit
    // the terminal
    for (i, frame) in frames.enumerate() {
        let mut image = frame.into_buffer();

        // display the image:
        //render_image(image.convert(), width, height);

        println!("Frame: {}", i);
        thread::sleep(time::Duration::from_millis(100));
        println!("next frame...");
    }
}
