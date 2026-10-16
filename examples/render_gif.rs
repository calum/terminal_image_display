//! Renders a gif (or any image) the way termage shows it in a terminal, and
//! saves the result as a new gif. That is how the README logo is made.
//!
//! ```text
//! cargo run --example render_gif -- INPUT OUTPUT [COLS] [ROWS] [SCALE]
//! ```
//!
//! `INPUT` is a file or web address. The picture is fitted into `COLS` by
//! `ROWS` terminal cells (default 80 by 30). Every cell is drawn as a
//! `SCALE` wide (default 8) block, twice as tall, split into its two halves.

use std::fs::File;
use std::process::ExitCode;

use image::codecs::gif::{GifEncoder, Repeat};
use image::{Delay, Frame as GifFrame, Rgb, RgbImage, RgbaImage};
use termage::{load_frames, load_image, read, to_display, Frame};
use terminal_graphics::Display;

/// Draws a display as pixels: each cell is `scale` wide and `2 * scale`
/// tall, its background on top and its foreground below, which is exactly
/// what a `▄` character looks like.
fn rasterise(display: &Display, scale: u32) -> RgbaImage {
    let (cols, rows) = display.size();
    let mut image = RgbImage::new(cols * scale, rows * scale * 2);
    for row in 0..rows {
        for col in 0..cols {
            let Some(cell) = display.pixel(col as isize, row as isize) else {
                continue;
            };
            let (top, bottom) = (cell.background().rgb(), cell.colour().rgb());
            for dy in 0..scale * 2 {
                let (r, g, b) = if dy < scale { top } else { bottom };
                for dx in 0..scale {
                    image.put_pixel(col * scale + dx, row * scale * 2 + dy, Rgb([r, g, b]));
                }
            }
        }
    }
    image::DynamicImage::ImageRgb8(image).to_rgba8()
}

fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let [input, output, rest @ ..] = args else {
        return Err("usage: render_gif INPUT OUTPUT [COLS] [ROWS] [SCALE]".into());
    };
    let number = |index: usize, default: u32| -> Result<u32, Box<dyn std::error::Error>> {
        rest.get(index)
            .map_or(Ok(default), |text| Ok(text.parse::<u32>()?.max(1)))
    };
    let (cols, rows, scale) = (number(0, 80)?, number(1, 30)?, number(2, 8)?);

    let bytes = read(input)?;
    let frames = if termage::is_gif(&bytes) {
        load_frames(&bytes)?
    } else {
        vec![Frame {
            image: load_image(&bytes)?,
            delay: std::time::Duration::from_millis(100),
        }]
    };

    let mut encoder = GifEncoder::new(File::create(output)?);
    encoder.set_repeat(Repeat::Infinite)?;
    for frame in &frames {
        let picture = rasterise(&to_display(&frame.image, cols, rows), scale);
        let delay = Delay::from_numer_denom_ms(frame.delay.as_millis() as u32, 1);
        encoder.encode_frame(GifFrame::from_parts(picture, 0, 0, delay))?;
    }
    println!("wrote {} frames to {output}", frames.len());
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("render_gif: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;
    use terminal_graphics::Colour;

    #[test]
    fn a_cell_is_drawn_background_on_top_and_foreground_below() {
        let mut display = Display::new(1, 1);
        display.set_pixel(0, 0, '▄', Colour::BrightBlue, Colour::BrightRed);

        let image = rasterise(&display, 2);
        assert_eq!(image.dimensions(), (2, 4));
        let red = Colour::BrightRed.rgb();
        let blue = Colour::BrightBlue.rgb();
        assert_eq!(image.get_pixel(1, 1), &Rgba([red.0, red.1, red.2, 255]));
        assert_eq!(image.get_pixel(0, 2), &Rgba([blue.0, blue.1, blue.2, 255]));
    }

    #[test]
    fn missing_arguments_are_a_usage_error() {
        assert!(run(&[]).is_err());
        assert!(run(&["only-input.png".to_string()]).is_err());
    }

    #[test]
    fn a_picture_is_turned_into_a_gif_file() {
        let dir = std::env::temp_dir().join(format!("termage_render_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let input = dir.join("in.png");
        let output = dir.join("out.gif");
        RgbImage::from_pixel(8, 8, Rgb([255, 0, 0]))
            .save(&input)
            .unwrap();

        let args = [input, output.clone()].map(|p| p.to_str().unwrap().to_string());
        run(&[
            args[0].clone(),
            args[1].clone(),
            "4".into(),
            "2".into(),
            "2".into(),
        ])
        .unwrap();

        let gif = std::fs::read(&output).unwrap();
        assert!(termage::is_gif(&gif));
        let frames = load_frames(&gif).unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].image.dimensions(), (8, 8));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
