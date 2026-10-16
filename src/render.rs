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
    build(image, max_cols, max_rows, false)
}

/// Like [`to_display`], but spreads the colour error over neighbouring pixels
/// (Floyd-Steinberg dithering). With only 16 colours to choose from this
/// turns flat bands in gradients, skies and photos into smooth looking
/// patterns, at the cost of some noise in flat areas.
///
/// ```
/// use image::{Rgb, RgbImage};
///
/// // this grey is between two of the 16 colours, so it is made from both
/// let grey = RgbImage::from_pixel(8, 8, Rgb([106, 106, 106]));
/// let plain = termage::to_display(&grey, 8, 4);
/// let dithered = termage::to_display_dithered(&grey, 8, 4);
///
/// let colours = |display: &terminal_graphics::Display| {
///     let mut seen = Vec::new();
///     for y in 0..display.height() as isize {
///         for x in 0..display.width() as isize {
///             let cell = display.pixel(x, y).unwrap();
///             for colour in [cell.colour(), cell.background()] {
///                 if !seen.contains(&colour) {
///                     seen.push(colour);
///                 }
///             }
///         }
///     }
///     seen.len()
/// };
/// assert_eq!(colours(&plain), 1);
/// assert!(colours(&dithered) > 1);
/// ```
pub fn to_display_dithered(image: &RgbImage, max_cols: u32, max_rows: u32) -> Display {
    build(image, max_cols, max_rows, true)
}

fn build(image: &RgbImage, max_cols: u32, max_rows: u32, dither: bool) -> Display {
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

    let colours = if dither {
        dithered_colours(&scaled)
    } else {
        scaled
            .pixels()
            .map(|pixel| Colour::from_rgb(pixel[0], pixel[1], pixel[2]))
            .collect()
    };

    let mut display = Display::new(width, height / 2);
    for row in 0..height / 2 {
        for x in 0..width {
            let at = |y: u32| colours[(y * width + x) as usize];
            display.set_pixel(
                x as isize,
                row as isize,
                HALF_BLOCK,
                at(row * 2 + 1),
                at(row * 2),
            );
        }
    }
    display
}

/// Picks a colour for every pixel (left to right, top to bottom) with
/// Floyd-Steinberg error diffusion: whatever a pixel's colour got wrong is
/// handed on to the pixels beside and below it, so that on average they come
/// out right. A flat patch of a colour that is in the palette stays flat.
fn dithered_colours(image: &RgbImage) -> Vec<Colour> {
    let (width, height) = (image.width() as usize, image.height() as usize);
    let mut pending: Vec<[f32; 3]> = image.pixels().map(|pixel| pixel.0.map(f32::from)).collect();
    let mut colours = Vec::with_capacity(width * height);

    for y in 0..height {
        for x in 0..width {
            let wanted = pending[y * width + x];
            let [red, green, blue] = wanted.map(|channel| channel.clamp(0.0, 255.0) as u8);
            let colour = Colour::from_rgb(red, green, blue);
            colours.push(colour);

            let (r, g, b) = colour.rgb();
            let error = [
                wanted[0] - f32::from(r),
                wanted[1] - f32::from(g),
                wanted[2] - f32::from(b),
            ];
            for (dx, dy, share) in [(1, 0, 7.0), (-1, 1, 3.0), (0, 1, 5.0), (1, 1, 1.0)] {
                let (nx, ny) = (x as isize + dx, y + dy as usize);
                if nx < 0 || nx >= width as isize || ny >= height {
                    continue;
                }
                let target = &mut pending[ny * width + nx as usize];
                for channel in 0..3 {
                    target[channel] += error[channel] * share / 16.0;
                }
            }
        }
    }
    colours
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

    fn distinct_colours(display: &Display) -> usize {
        let mut seen = Vec::new();
        for y in 0..display.height() as isize {
            for x in 0..display.width() as isize {
                let cell = display.pixel(x, y).unwrap();
                for colour in [cell.colour(), cell.background()] {
                    if !seen.contains(&colour) {
                        seen.push(colour);
                    }
                }
            }
        }
        seen.len()
    }

    #[test]
    fn dithering_mixes_colours_to_approximate_one_in_between() {
        let grey = RgbImage::from_pixel(16, 16, Rgb([106, 106, 106]));
        assert_eq!(distinct_colours(&to_display(&grey, 16, 8)), 1);
        assert!(distinct_colours(&to_display_dithered(&grey, 16, 8)) >= 2);
    }

    #[test]
    fn dithering_leaves_black_and_white_alone() {
        for colour in [[0, 0, 0], [255, 255, 255]] {
            let flat = RgbImage::from_pixel(8, 8, Rgb(colour));
            let plain = to_display(&flat, 8, 4);
            assert_eq!(to_display_dithered(&flat, 8, 4), plain, "{colour:?}");
        }
    }

    #[test]
    fn dithering_does_not_change_the_size() {
        let image = RgbImage::from_pixel(40, 30, Rgb([90, 160, 40]));
        assert_eq!(
            to_display_dithered(&image, 20, 10).size(),
            to_display(&image, 20, 10).size()
        );
    }

    #[test]
    fn a_dithered_gradient_uses_more_colours_than_a_plain_one() {
        let mut gradient = RgbImage::new(64, 8);
        for (x, _, pixel) in gradient.enumerate_pixels_mut() {
            let v = (x * 4) as u8;
            *pixel = Rgb([v, v / 2, 255 - v]);
        }
        assert!(
            distinct_colours(&to_display_dithered(&gradient, 64, 4))
                > distinct_colours(&to_display(&gradient, 64, 4))
        );
    }
}
