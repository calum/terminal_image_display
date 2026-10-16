//! Reading images and animated gifs from disk.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
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

/// Whether the file at `path` is a gif, going by its contents rather than its
/// name.
pub fn is_gif(path: &Path) -> Result<bool, Error> {
    let reader = ImageReader::open(path)?.with_guessed_format()?;
    Ok(reader.format() == Some(ImageFormat::Gif))
}

/// Loads any still image the `image` crate can decode (PNG, JPEG, GIF, BMP
/// and WebP). Transparent pixels become black. For an animated gif this is
/// the first frame.
pub fn load_image(path: &Path) -> Result<RgbImage, Error> {
    let image = ImageReader::open(path)?.with_guessed_format()?.decode()?;
    Ok(flatten(&image.to_rgba8()))
}

/// Loads every frame of an animated gif.
pub fn load_frames(path: &Path) -> Result<Vec<Frame>, Error> {
    let decoder = GifDecoder::new(BufReader::new(File::open(path)?))?;
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

/// Blends transparent pixels onto black.
fn flatten(image: &RgbaImage) -> RgbImage {
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
    fn a_missing_file_is_an_io_error() {
        let path = Path::new("definitely/not/here.png");
        assert!(matches!(load_image(path), Err(Error::Io(_))));
        assert!(matches!(load_frames(path), Err(Error::Io(_))));
        assert!(matches!(is_gif(path), Err(Error::Io(_))));
    }
}
