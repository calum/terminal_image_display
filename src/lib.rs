//! View images, animated gifs and videos in the terminal, drawn with
//! [`terminal_graphics`].
//!
//! Every terminal cell shows two image pixels using a half block character, so
//! a picture keeps its proportions. Colours are matched to the 16 ANSI colours.
//! This crate backs the `termage` command line tool (try it in your browser at
//! <https://calum.github.io/terminal_image_display/>), and you can use it from
//! your own program too:
//!
//! ```no_run
//! use termage::Options;
//!
//! // show a picture, play a gif, or play a video (needs ffmpeg), from a
//! // file or a web address, in an 80 column by 24 row area
//! termage::show(&mut std::io::stdout(), "ferris.png", &Options::new(80, 24))?;
//! termage::show(&mut std::io::stdout(), "https://example.com/cat.gif", &Options::new(80, 24))?;
//! # Ok::<(), termage::Error>(())
//! ```
//!
//! The pieces are public too: [`load_image`] and [`load_frames`] decode
//! bytes, [`to_display`] turns an [`image::RgbImage`] into a
//! [`terminal_graphics::Display`], and [`fit`] works out how big the picture
//! will be.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod error;
mod load;
mod render;
mod source;
mod video;

use std::io::{self, Write};
use std::thread;

pub use error::Error;
pub use load::{flatten, is_gif, load_frames, load_image, Frame};
pub use render::{fit, to_display, to_display_dithered};
pub use source::{extension, is_url, read};
pub use video::{is_video, play_video, DEFAULT_FPS};

/// How and where to draw.
///
/// Build one with [`Options::new`] and the `with_*` methods:
///
/// ```
/// let options = termage::Options::new(80, 24).with_loops(Some(2)).with_dither(true);
/// assert_eq!(options.loops, Some(2));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Options {
    /// The most columns to use.
    pub cols: u32,
    /// The most rows to use.
    pub rows: u32,
    /// How many times to play a gif or video. `None` means until stopped for
    /// a gif, and once for a video.
    pub loops: Option<u32>,
    /// Frames per second for video.
    pub fps: u32,
    /// Whether to dither, see [`to_display_dithered`].
    pub dither: bool,
}

impl Options {
    /// Options for a `cols` by `rows` area, with the other settings at their
    /// defaults.
    pub fn new(cols: u32, rows: u32) -> Options {
        Options {
            cols,
            rows,
            loops: None,
            fps: DEFAULT_FPS,
            dither: false,
        }
    }

    /// Sets how many times a gif or video plays, see [`Options::loops`].
    pub fn with_loops(mut self, loops: Option<u32>) -> Options {
        self.loops = loops;
        self
    }

    /// Sets the frames per second used for video.
    pub fn with_fps(mut self, fps: u32) -> Options {
        self.fps = fps;
        self
    }

    /// Turns dithering on or off.
    pub fn with_dither(mut self, dither: bool) -> Options {
        self.dither = dither;
        self
    }

    /// Scales and draws `image` according to these options.
    pub fn display(&self, image: &image::RgbImage) -> terminal_graphics::Display {
        if self.dither {
            to_display_dithered(image, self.cols, self.rows)
        } else {
            to_display(image, self.cols, self.rows)
        }
    }
}

/// Used when the terminal size is unknown, for example when piping output.
pub const DEFAULT_SIZE: (u32, u32) = (80, 24);

/// Works out how many columns and rows to draw into.
///
/// `width` and `height` are what the user asked for. Anything missing comes
/// from the `terminal` size, or from [`DEFAULT_SIZE`] when that is unknown.
/// When the height comes from the terminal one row is left free for the
/// shell prompt. The result is never smaller than 1 by 1.
///
/// ```
/// // an explicit width wins; the height is the terminal's, less a prompt row
/// assert_eq!(termage::pick_size(Some(40), None, Some((120, 50))), (40, 49));
/// assert_eq!(termage::pick_size(None, None, None), termage::DEFAULT_SIZE);
/// ```
pub fn pick_size(
    width: Option<u32>,
    height: Option<u32>,
    terminal: Option<(u16, u16)>,
) -> (u32, u32) {
    let (terminal_cols, terminal_rows) = match terminal {
        Some((cols, rows)) => (u32::from(cols), u32::from(rows).saturating_sub(1)),
        None => DEFAULT_SIZE,
    };
    (
        width.unwrap_or(terminal_cols).max(1),
        height.unwrap_or(terminal_rows).max(1),
    )
}

/// Shows the image, animated gif or video at `input` in `out`.
///
/// `input` is a file path or an `http://` / `https://` address. A still image
/// is drawn once. A gif with several frames is played `options.loops` times,
/// or until the program is stopped if that is `None`. A video (recognised by
/// its file extension, see [`is_video`]) is played with `ffmpeg`, once unless
/// `options.loops` says otherwise.
///
/// Output is ANSI escape codes, so `out` is normally `stdout`.
pub fn show<W: Write>(out: &mut W, input: &str, options: &Options) -> Result<(), Error> {
    if is_video(input) {
        return play_video(out, input, options);
    }

    let bytes = read(input)?;
    if is_gif(&bytes) {
        let frames = load_frames(&bytes)?;
        play(out, &frames, options)?;
    } else {
        let image = load_image(&bytes)?;
        show_image(out, &image, options)?;
    }
    Ok(())
}

/// Draws one picture into `out`, below whatever is already there.
pub fn show_image<W: Write>(
    out: &mut W,
    image: &image::RgbImage,
    options: &Options,
) -> io::Result<()> {
    let display = options.display(image);
    reserve_space(out, &display)?;
    display.write_to(out)?;
    out.flush()
}

/// Plays `frames` in place, `options.loops` times (forever if `None`),
/// waiting each frame's delay. A single frame is just drawn once.
pub fn play<W: Write>(out: &mut W, frames: &[Frame], options: &Options) -> io::Result<()> {
    // scale every frame once up front rather than on every loop
    let displays: Vec<_> = frames
        .iter()
        .map(|frame| (options.display(&frame.image), frame.delay))
        .collect();
    let Some((first, _)) = displays.first() else {
        return Ok(());
    };
    reserve_space(out, first)?;

    let mut played = 0;
    loop {
        for (display, delay) in &displays {
            display.write_to(out)?;
            out.flush()?;
            if displays.len() > 1 {
                thread::sleep(*delay);
            }
        }
        played += 1;
        if displays.len() == 1 || options.loops.is_some_and(|loops| played >= loops) {
            return Ok(());
        }
    }
}

/// Every frame starts by moving the cursor up over the last one, so make
/// room for it first.
fn reserve_space<W: Write>(out: &mut W, display: &terminal_graphics::Display) -> io::Result<()> {
    out.write_all("\n".repeat(display.height() as usize).as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};
    use std::time::Duration;

    fn solid(colour: [u8; 3]) -> Frame {
        Frame {
            image: RgbImage::from_pixel(4, 4, Rgb(colour)),
            delay: Duration::from_millis(1),
        }
    }

    #[test]
    fn pick_size_prefers_what_was_asked_for() {
        assert_eq!(pick_size(Some(10), Some(5), Some((100, 50))), (10, 5));
    }

    #[test]
    fn pick_size_leaves_a_row_for_the_prompt() {
        assert_eq!(pick_size(None, None, Some((100, 50))), (100, 49));
    }

    #[test]
    fn pick_size_falls_back_without_a_terminal() {
        assert_eq!(pick_size(None, None, None), DEFAULT_SIZE);
    }

    #[test]
    fn pick_size_is_never_zero() {
        assert_eq!(pick_size(Some(0), Some(0), None), (1, 1));
        assert_eq!(pick_size(None, None, Some((0, 0))), (1, 1));
    }

    #[test]
    fn a_still_image_is_drawn_once_below_reserved_space() {
        let image = RgbImage::from_pixel(4, 4, Rgb([255, 0, 0]));
        let mut out = Vec::new();
        show_image(&mut out, &image, &Options::new(10, 10)).unwrap();

        let text = String::from_utf8(out).unwrap();
        // 4x4 pixels scale to 20 columns... capped by the 10 columns we gave it
        assert!(text.starts_with(&"\n".repeat(5)));
        assert_eq!(text.matches('▄').count(), 10 * 5);
    }

    #[test]
    fn frames_play_the_requested_number_of_times() {
        let frames = [solid([255, 0, 0]), solid([0, 0, 255])];
        let mut out = Vec::new();
        play(&mut out, &frames, &Options::new(2, 1).with_loops(Some(3))).unwrap();

        let text = String::from_utf8(out).unwrap();
        // 2 frames x 3 loops, each of 2 cells
        assert_eq!(text.matches('▄').count(), 2 * 3 * 2);
        // space is reserved once, not per frame
        assert_eq!(text.matches('\n').count(), 1);
    }

    #[test]
    fn a_single_frame_is_not_looped() {
        let mut out = Vec::new();
        play(&mut out, &[solid([0, 255, 0])], &Options::new(2, 1)).unwrap();
        assert_eq!(String::from_utf8(out).unwrap().matches('▄').count(), 2);
    }

    #[test]
    fn no_frames_draws_nothing() {
        let mut out = Vec::new();
        play(&mut out, &[], &Options::new(10, 10)).unwrap();
        assert!(out.is_empty());
    }
}

// compile-check the code blocks in the README, so it can't go stale
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
