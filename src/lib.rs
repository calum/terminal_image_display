extern crate image;

use std::path::Path;

use image::{GenericImage, Pixel, RgbImage};

pub fn pixelate_image(img: &mut RgbImage, width: u32, height: u32) -> RgbImage {
    // Create a new ImgBuf
    let mut imgbuf = image::ImageBuffer::new(width, height);

    closest_match(&mut imgbuf, img);

    imgbuf
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
