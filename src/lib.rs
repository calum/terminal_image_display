extern crate image;

use std::fs::File;
use std::path::Path;

use image::{GenericImage, SubImage, Pixel, ImageBuffer, RgbImage};

pub fn pixelate_image(img: &mut RgbImage, width: u32, height: u32) -> RgbImage {
    // Create a new ImgBuf
    let mut imgbuf = image::ImageBuffer::new(width, height);

    merge_pixels(&mut imgbuf, img);

    imgbuf
}

pub fn get_image(filename: &str) -> RgbImage {
    let mut img = image::open(&Path::new(filename)).unwrap();

    img.to_rgb()
}

pub fn merge_pixels(imgbuf: &mut RgbImage, img: &mut RgbImage) {
    // The dimensions method returns the images width and height
    let (width, height) = img.dimensions();
    let (out_width, out_height) = imgbuf.dimensions();

    // set the variables needed to average out the pixels
    let scale_x = width/out_width;
    let scale_y = height/(out_height);
    let num_pixels = (scale_y*scale_y) as f32;

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
