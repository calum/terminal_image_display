//! Turning an image into a [`Display`] of half-block characters.
//!
//! Each terminal cell shows two image pixels, one above the other: the cell is
//! a `▄` (lower half block) whose foreground colour is the lower pixel and
//! whose background colour is the upper pixel. Since a cell is about twice as
//! tall as it is wide, the image pixels come out roughly square.

use image::imageops::{self, FilterType};
use image::RgbImage;
use terminal_graphics::{Colour, Display};

/// The character drawn in every cell.
const HALF_BLOCK: char = '▄';

/// The size, in image pixels, an image of `size` is scaled to so that it fits
/// in `max_cols` by `max_rows` terminal cells, keeping its proportions.
///
/// Returns `(width, height)` where `width` is also the number of columns, and
/// `height` is even (two pixels per row). Small images are scaled up. The
/// result is never smaller than 1 column by 2 pixels, whatever the input.
///
/// ```
/// // a square image on a wide terminal is limited by the height:
/// // 10 rows are 20 pixels tall, so it comes out 20 pixels wide
/// assert_eq!(termage::fit((100, 100), 80, 10), (20, 20));
/// ```
pub fn fit(size: (u32, u32), max_cols: u32, max_rows: u32) -> (u32, u32) {
    let (width, height) = (size.0.max(1) as f64, size.1.max(1) as f64);
    let max_width = max_cols.max(1) as f64;
    let max_height = (max_rows.max(1) as f64) * 2.0;

    let scale = (max_width / width).min(max_height / height);
    let out_width = (width * scale).round().clamp(1.0, max_width) as u32;
    let mut out_height = (height * scale).round().clamp(2.0, max_height) as u32;
    // two pixels per row, so the height must be even
    out_height -= out_height % 2;
    (out_width, out_height.max(2))
}

/// Scales `image` to fit `max_cols` by `max_rows` cells and draws it into a
/// new [`Display`] that is exactly as big as the scaled image.
///
/// Colours are matched to the nearest of the 16 terminal colours with
/// [`Colour::from_rgb`].
///
/// ```
/// use image::{Rgb, RgbImage};
/// use terminal_graphics::Colour;
///
/// // a red pixel above a blue one makes one cell
/// let mut image = RgbImage::new(1, 2);
/// image.put_pixel(0, 0, Rgb([255, 0, 0]));
/// image.put_pixel(0, 1, Rgb([0, 0, 255]));
///
/// let display = termage::to_display(&image, 1, 1);
/// let cell = display.pixel(0, 0).unwrap();
/// assert_eq!(cell.background(), Colour::BrightRed);
/// assert_eq!(cell.colour(), Colour::BrightBlue);
/// ```
pub fn to_display(image: &RgbImage, max_cols: u32, max_rows: u32) -> Display {
    let (width, height) = fit(image.dimensions(), max_cols, max_rows);
    let (source_width, source_height) = image.dimensions();
    // nearest keeps pixel art crisp when scaling up; averaging looks right when scaling down
    let filter = if width > source_width || height > source_height {
        FilterType::Nearest
    } else {
        FilterType::Triangle
    };
    let scaled = if (width, height) == (source_width, source_height) {
        image.clone()
    } else {
        imageops::resize(image, width, height, filter)
    };

    let mut display = Display::new(width, height / 2);
    for row in 0..height / 2 {
        for x in 0..width {
            let upper = colour_of(&scaled, x, row * 2);
            let lower = colour_of(&scaled, x, row * 2 + 1);
            display.set_pixel(x as isize, row as isize, HALF_BLOCK, lower, upper);
        }
    }
    display
}

fn colour_of(image: &RgbImage, x: u32, y: u32) -> Colour {
    let [red, green, blue] = image.get_pixel(x, y).0;
    Colour::from_rgb(red, green, blue)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgb;

    #[test]
    fn fit_is_limited_by_the_tighter_dimension() {
        // wide image: limited by the columns
        assert_eq!(fit((200, 100), 40, 40), (40, 20));
        // tall image: limited by the rows (20 rows = 40 pixels)
        assert_eq!(fit((100, 200), 80, 20), (20, 40));
    }

    #[test]
    fn fit_scales_small_images_up() {
        assert_eq!(fit((4, 4), 40, 20), (40, 40));
    }

    #[test]
    fn fit_never_exceeds_the_terminal_and_always_has_even_height() {
        for &(w, h) in &[(1, 1), (3, 7), (640, 480), (1, 5000), (5000, 1), (97, 31)] {
            for &(cols, rows) in &[(1, 1), (10, 3), (80, 24), (200, 60)] {
                let (out_w, out_h) = fit((w, h), cols, rows);
                assert!(out_w >= 1 && out_w <= cols, "{w}x{h} in {cols}x{rows}");
                assert!(out_h >= 2 && out_h <= rows * 2, "{w}x{h} in {cols}x{rows}");
                assert_eq!(out_h % 2, 0);
            }
        }
    }

    #[test]
    fn fit_copes_with_zero_sizes() {
        assert_eq!(fit((0, 0), 0, 0), (1, 2));
    }

    #[test]
    fn fit_keeps_the_aspect_ratio_roughly() {
        let (w, h) = fit((1920, 1080), 100, 100);
        let ratio = w as f64 / h as f64;
        assert!((ratio - 16.0 / 9.0).abs() < 0.05, "ratio {ratio}");
    }

    #[test]
    fn each_cell_shows_two_pixels_with_the_lower_one_in_front() {
        let mut image = RgbImage::new(2, 2);
        image.put_pixel(0, 0, Rgb([255, 0, 0]));
        image.put_pixel(0, 1, Rgb([0, 255, 0]));
        image.put_pixel(1, 0, Rgb([0, 0, 255]));
        image.put_pixel(1, 1, Rgb([255, 255, 0]));

        let display = to_display(&image, 2, 1);
        assert_eq!(display.size(), (2, 1));

        let left = display.pixel(0, 0).unwrap();
        assert_eq!(left.character(), '▄');
        assert_eq!(left.background(), Colour::BrightRed);
        assert_eq!(left.colour(), Colour::BrightGreen);

        let right = display.pixel(1, 0).unwrap();
        assert_eq!(right.background(), Colour::BrightBlue);
        assert_eq!(right.colour(), Colour::BrightYellow);
    }

    #[test]
    fn the_display_is_as_big_as_the_scaled_image() {
        let image = RgbImage::new(100, 50);
        let display = to_display(&image, 20, 20);
        assert_eq!(display.size(), (20, 5));
    }

    #[test]
    fn downscaling_averages_instead_of_dropping_pixels() {
        // a black and white checkerboard averages to grey, not to black or white
        let mut image = RgbImage::new(8, 8);
        for (x, y, pixel) in image.enumerate_pixels_mut() {
            *pixel = if (x + y) % 2 == 0 {
                Rgb([255; 3])
            } else {
                Rgb([0; 3])
            };
        }
        let display = to_display(&image, 2, 1);
        let cell = display.pixel(0, 0).unwrap();
        assert!(![Colour::Black, Colour::BrightWhite].contains(&cell.colour()));
    }

    #[test]
    fn a_one_pixel_image_still_draws() {
        let display = to_display(&RgbImage::new(1, 1), 10, 10);
        assert!(display.width() >= 1 && display.height() >= 1);
    }
}
