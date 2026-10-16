//! Reading images and animated gifs.

use std::io::Cursor;
use std::time::Duration;

use image::codecs::gif::GifDecoder;
use image::{AnimationDecoder, ImageFormat, ImageReader, Rgb, RgbImage, RgbaImage};

use crate::Error;

/// Gif delays this short are treated as "unspecified" and become
/// [`DEFAULT_DELAY`], the same as browsers do.
const MIN_DELAY: Duration = Duration::from_millis(20);
const DEFAULT_DELAY: Duration = Duration::from_millis(100);

/// One frame of an animated gif: what to show and for how long.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    /// The picture, with transparency flattened onto black.
    pub image: RgbImage,
    /// How long to leave it on screen.
    pub delay: Duration,
}

/// Whether `bytes` (the contents of a file) are a gif, going by the contents
/// rather than a file name.
pub fn is_gif(bytes: &[u8]) -> bool {
    image::guess_format(bytes).is_ok_and(|format| format == ImageFormat::Gif)
}

/// Decodes any still image the `image` crate can read (PNG, JPEG, GIF, BMP
/// and WebP). Transparent pixels become black. For an animated gif this is
/// the first frame.
pub fn load_image(bytes: &[u8]) -> Result<RgbImage, Error> {
    let image = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()?
        .decode()?;
    Ok(flatten(&image.to_rgba8()))
}

/// Decodes every frame of an animated gif.
pub fn load_frames(bytes: &[u8]) -> Result<Vec<Frame>, Error> {
    let decoder = GifDecoder::new(Cursor::new(bytes))?;
    let frames = decoder.into_frames().collect_frames()?;
    Ok(frames
        .into_iter()
        .map(|frame| Frame {
            delay: delay_of(frame.delay().numer_denom_ms()),
            image: flatten(frame.buffer()),
        })
        .collect())
}

fn delay_of((numerator, denominator): (u32, u32)) -> Duration {
    let delay = Duration::from_millis(u64::from(numerator) / u64::from(denominator.max(1)));
    if delay < MIN_DELAY {
        DEFAULT_DELAY
    } else {
        delay
    }
}

/// Blends transparent pixels onto black, giving a picture with no alpha.
pub fn flatten(image: &RgbaImage) -> RgbImage {
    let mut flat = RgbImage::new(image.width(), image.height());
    for (x, y, pixel) in image.enumerate_pixels() {
        let [red, green, blue, alpha] = pixel.0;
        let blend = |channel: u8| (u16::from(channel) * u16::from(alpha) / 255) as u8;
        flat.put_pixel(x, y, Rgb([blend(red), blend(green), blend(blue)]));
    }
    flat
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn transparent_pixels_become_black_and_opaque_ones_are_kept() {
        let mut image = RgbaImage::new(3, 1);
        image.put_pixel(0, 0, Rgba([200, 100, 50, 255]));
        image.put_pixel(1, 0, Rgba([200, 100, 50, 0]));
        image.put_pixel(2, 0, Rgba([200, 100, 50, 128]));

        let flat = flatten(&image);
        assert_eq!(flat.get_pixel(0, 0), &Rgb([200, 100, 50]));
        assert_eq!(flat.get_pixel(1, 0), &Rgb([0, 0, 0]));
        assert_eq!(flat.get_pixel(2, 0), &Rgb([100, 50, 25]));
    }

    #[test]
    fn short_or_missing_gif_delays_use_the_default() {
        assert_eq!(delay_of((0, 1)), DEFAULT_DELAY);
        assert_eq!(delay_of((10, 1)), DEFAULT_DELAY);
        assert_eq!(delay_of((100, 0)), Duration::from_millis(100));
    }

    #[test]
    fn longer_delays_are_kept() {
        assert_eq!(delay_of((20, 1)), Duration::from_millis(20));
        assert_eq!(delay_of((500, 1)), Duration::from_millis(500));
        assert_eq!(delay_of((1000, 2)), Duration::from_millis(500));
    }

    #[test]
    fn bytes_that_are_not_an_image_are_a_decode_error() {
        let junk = b"this is not a picture";
        assert!(!is_gif(junk));
        assert!(load_image(junk).is_err());
        assert!(load_frames(junk).is_err());
    }

    #[test]
    fn a_gif_header_is_recognised() {
        assert!(is_gif(b"GIF89a\x01\x00\x01\x00"));
        assert!(!is_gif(b"\x89PNG\r\n\x1a\n"));
    }
}
