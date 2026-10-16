//! Draws the sample pictures used by the README and the website.
//!
//! ```text
//! cargo run --example make_samples -- [OUT_DIR]     # default docs/samples
//! ```
//!
//! Everything is generated from code, so there are no stock photos to license
//! and the samples can be redrawn at will. Between them they cover what termage
//! handles: a transparent PNG, a JPEG, a detailed PNG, and two animated gifs.

use std::f32::consts::PI;
use std::fs::{self, File};
use std::path::Path;
use std::process::ExitCode;

use image::codecs::gif::{GifEncoder, Repeat};
use image::{Delay, Frame, Rgba, RgbaImage};

type Colour = [u8; 3];

fn rgba(colour: Colour) -> Rgba<u8> {
    Rgba([colour[0], colour[1], colour[2], 255])
}

/// Converts hue (0 to 1, wrapping), saturation and value (0 to 1) to RGB.
fn hsv(hue: f32, saturation: f32, value: f32) -> Colour {
    let h = hue.rem_euclid(1.0) * 6.0;
    let (sector, fraction) = (h.floor() as u32, h - h.floor());
    let (p, q, t) = (
        value * (1.0 - saturation),
        value * (1.0 - saturation * fraction),
        value * (1.0 - saturation * (1.0 - fraction)),
    );
    let (r, g, b) = match sector {
        0 => (value, t, p),
        1 => (q, value, p),
        2 => (p, value, t),
        3 => (p, q, value),
        4 => (t, p, value),
        _ => (value, p, q),
    };
    [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8]
}

fn mix(a: Colour, b: Colour, amount: f32) -> Colour {
    let amount = amount.clamp(0.0, 1.0);
    [0, 1, 2].map(|i| (f32::from(a[i]) * (1.0 - amount) + f32::from(b[i]) * amount) as u8)
}

fn fill_ellipse(image: &mut RgbaImage, cx: f32, cy: f32, rx: f32, ry: f32, colour: Colour) {
    for y in 0..image.height() {
        for x in 0..image.width() {
            let (dx, dy) = ((x as f32 - cx) / rx, (y as f32 - cy) / ry);
            if dx * dx + dy * dy <= 1.0 {
                image.put_pixel(x, y, rgba(colour));
            }
        }
    }
}

/// A thick line, drawn as a row of discs.
fn draw_line(image: &mut RgbaImage, from: (f32, f32), to: (f32, f32), width: f32, colour: Colour) {
    let steps = ((to.0 - from.0).hypot(to.1 - from.1) * 2.0).max(1.0) as u32;
    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        let (x, y) = (from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t);
        fill_ellipse(image, x, y, width / 2.0, width / 2.0, colour);
    }
}

/// A crab on a transparent background, to show that transparency works.
pub fn crab() -> RgbaImage {
    let mut image = RgbaImage::new(240, 180);
    let shell = [222, 88, 48];
    let dark = [150, 50, 30];

    for side in [-1.0f32, 1.0] {
        let x = |offset: f32| 120.0 + side * offset;
        // legs, then arms and claws
        for (start, end) in [
            ((40.0, 118.0), (95.0, 160.0)),
            ((55.0, 128.0), (80.0, 172.0)),
        ] {
            draw_line(
                &mut image,
                (x(start.0), start.1),
                (x(end.0), end.1),
                9.0,
                dark,
            );
        }
        draw_line(&mut image, (x(55.0), 90.0), (x(92.0), 55.0), 11.0, shell);
        fill_ellipse(&mut image, x(98.0), 48.0, 24.0, 20.0, shell);
        fill_ellipse(&mut image, x(108.0), 40.0, 9.0, 8.0, dark);
        // eye stalks
        draw_line(&mut image, (x(24.0), 70.0), (x(26.0), 48.0), 5.0, dark);
        fill_ellipse(&mut image, x(26.0), 44.0, 13.0, 13.0, [255, 255, 255]);
        fill_ellipse(&mut image, x(28.0), 46.0, 6.0, 6.0, [20, 20, 20]);
    }
    fill_ellipse(&mut image, 120.0, 100.0, 72.0, 48.0, shell);
    fill_ellipse(&mut image, 120.0, 118.0, 56.0, 24.0, [240, 140, 90]);
    // a smile
    for step in 0..=24 {
        let angle = PI * (0.2 + 0.6 * step as f32 / 24.0);
        let (x, y) = (120.0 + angle.cos() * 22.0, 96.0 + angle.sin() * 12.0);
        fill_ellipse(&mut image, x, y, 2.5, 2.5, [60, 20, 10]);
    }
    image
}

/// A sunset over mountains, saved as a JPEG.
pub fn sunset() -> RgbaImage {
    let (width, height) = (320u32, 200u32);
    let mut image = RgbaImage::new(width, height);
    let horizon = height as f32 * 0.68;
    for y in 0..height {
        for x in 0..width {
            let t = (y as f32 / horizon).min(1.0);
            let mut colour = mix(
                mix([25, 20, 70], [235, 90, 90], t),
                [255, 200, 110],
                t.powi(3),
            );
            let sun = ((x as f32 - 160.0).powi(2) + (y as f32 - horizon + 18.0).powi(2)).sqrt();
            if sun < 38.0 {
                colour = [255, 238, 170];
            } else {
                colour = mix(colour, [255, 190, 120], (1.0 - sun / 140.0).max(0.0) * 0.5);
            }
            let ridge = |scale: f32, phase: f32, base: f32, amplitude: f32| {
                base - amplitude
                    * ((x as f32 * scale + phase).sin() * 0.6
                        + (x as f32 * scale * 2.3 + phase * 1.7).sin() * 0.4)
            };
            if y as f32 > ridge(0.02, 1.0, horizon - 22.0, 22.0) {
                colour = [88, 42, 92];
            }
            if y as f32 > ridge(0.035, 4.0, horizon + 4.0, 16.0) {
                colour = [34, 16, 48];
            }
            image.put_pixel(x, y, rgba(colour));
        }
    }
    image
}

/// The Mandelbrot set, for fine detail and lots of colour.
pub fn mandelbrot() -> RgbaImage {
    let (width, height) = (320u32, 240u32);
    let mut image = RgbaImage::new(width, height);
    for py in 0..height {
        for px in 0..width {
            let cx = -2.2 + 3.2 * px as f32 / width as f32;
            let cy = -1.2 + 2.4 * py as f32 / height as f32;
            let (mut x, mut y, mut i) = (0.0f32, 0.0f32, 0u32);
            while x * x + y * y <= 4.0 && i < 80 {
                (x, y) = (x * x - y * y + cx, 2.0 * x * y + cy);
                i += 1;
            }
            let colour = if i == 80 {
                [0, 0, 0]
            } else {
                hsv(
                    0.62 + i as f32 * 0.035,
                    0.85,
                    0.35 + 0.65 * (i as f32 / 25.0).min(1.0),
                )
            };
            image.put_pixel(px, py, rgba(colour));
        }
    }
    image
}

/// Frames of a looping plasma, ready to encode.
pub fn plasma_frames(count: u32) -> Vec<RgbaImage> {
    let (width, height) = (96u32, 72u32);
    (0..count)
        .map(|frame| {
            let t = frame as f32 / count as f32 * 2.0 * PI;
            let mut image = RgbaImage::new(width, height);
            for y in 0..height {
                for x in 0..width {
                    let (fx, fy) = (x as f32 / 9.0, y as f32 / 9.0);
                    let value = (fx + t).sin()
                        + (fy * 1.3 - t).sin()
                        + ((fx + fy + t).sin() * 2.0).sin()
                        + ((fx * fx + fy * fy).sqrt() * 0.7 - t).sin();
                    image.put_pixel(x, y, rgba(hsv(value / 8.0 + 0.5, 0.9, 1.0)));
                }
            }
            image
        })
        .collect()
}

/// Frames of coloured balls bouncing round a dark room.
pub fn bounce_frames(count: u32) -> Vec<RgbaImage> {
    let (width, height) = (160u32, 120u32);
    let balls = [
        ([255, 90, 90], 1.0f32, 0.0f32),
        ([90, 220, 120], 2.0, 0.3),
        ([100, 150, 255], 3.0, 0.65),
        ([255, 220, 90], 1.0, 0.5),
    ];
    // a ball moving at constant speed between 0 and `limit`, bouncing off both ends
    let triangle = |t: f32, limit: f32| {
        let t = t.rem_euclid(2.0);
        limit * if t > 1.0 { 2.0 - t } else { t }
    };
    (0..count)
        .map(|frame| {
            let t = frame as f32 / count as f32;
            let mut image = RgbaImage::new(width, height);
            for y in 0..height {
                for x in 0..width {
                    let shade = 14.0 + 22.0 * y as f32 / height as f32;
                    let grid = if x % 20 == 0 || y % 20 == 0 {
                        10.0
                    } else {
                        0.0
                    };
                    let v = (shade + grid) as u8;
                    image.put_pixel(x, y, Rgba([v / 2, v / 2, v, 255]));
                }
            }
            for (colour, speed, offset) in balls {
                let x = 12.0 + triangle(t * speed * 2.0 + offset, width as f32 - 24.0);
                let y =
                    12.0 + triangle(t * (speed + 1.0) * 2.0 + offset * 3.0, height as f32 - 24.0);
                fill_ellipse(&mut image, x + 2.0, y + 3.0, 11.0, 11.0, [8, 8, 18]);
                fill_ellipse(&mut image, x, y, 11.0, 11.0, colour);
                fill_ellipse(
                    &mut image,
                    x - 3.0,
                    y - 3.0,
                    4.0,
                    4.0,
                    mix(colour, [255, 255, 255], 0.7),
                );
            }
            image
        })
        .collect()
}

fn save_gif(
    path: &Path,
    frames: &[RgbaImage],
    delay_ms: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut encoder = GifEncoder::new(File::create(path)?);
    encoder.set_repeat(Repeat::Infinite)?;
    for image in frames {
        let delay = Delay::from_numer_denom_ms(delay_ms, 1);
        encoder.encode_frame(Frame::from_parts(image.clone(), 0, 0, delay))?;
    }
    Ok(())
}

fn run(out: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(out)?;
    crab().save(out.join("crab.png"))?;
    mandelbrot().save(out.join("mandelbrot.png"))?;
    // JPEG has no alpha channel
    image::DynamicImage::ImageRgba8(sunset())
        .to_rgb8()
        .save(out.join("sunset.jpg"))?;
    save_gif(&out.join("plasma.gif"), &plasma_frames(30), 50)?;
    save_gif(&out.join("bounce.gif"), &bounce_frames(48), 40)?;
    println!("wrote the samples to {}", out.display());
    Ok(())
}

fn main() -> ExitCode {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "docs/samples".to_string());
    match run(Path::new(&out)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("make_samples: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hsv_hits_the_primary_colours() {
        assert_eq!(hsv(0.0, 1.0, 1.0), [255, 0, 0]);
        assert_eq!(hsv(1.0 / 3.0, 1.0, 1.0), [0, 255, 0]);
        assert_eq!(hsv(0.5, 0.0, 1.0), [255, 255, 255]);
        assert_eq!(hsv(0.7, 1.0, 0.0), [0, 0, 0]);
    }

    #[test]
    fn the_crab_is_transparent_around_it_and_solid_in_the_middle() {
        let image = crab();
        assert_eq!(image.get_pixel(0, 0).0[3], 0);
        assert_eq!(image.get_pixel(120, 100).0[3], 255);
    }

    #[test]
    fn the_still_pictures_have_the_expected_size() {
        assert_eq!(sunset().dimensions(), (320, 200));
        assert_eq!(mandelbrot().dimensions(), (320, 240));
    }

    #[test]
    fn the_gifs_have_distinct_frames() {
        for frames in [plasma_frames(6), bounce_frames(6)] {
            assert_eq!(frames.len(), 6);
            assert_ne!(frames[0], frames[3]);
        }
    }

    #[test]
    fn every_sample_is_written() {
        let dir = std::env::temp_dir().join(format!("termage_samples_{}", std::process::id()));
        run(&dir).unwrap();
        for name in [
            "crab.png",
            "mandelbrot.png",
            "sunset.jpg",
            "plasma.gif",
            "bounce.gif",
        ] {
            let bytes = fs::read(dir.join(name)).unwrap();
            assert!(image::guess_format(&bytes).is_ok(), "{name}");
        }
        fs::remove_dir_all(&dir).unwrap();
    }
}
