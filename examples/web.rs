//! The termage library compiled to WebAssembly, for the website's terminal.
//!
//! Build it with
//! `cargo build --release --example web --no-default-features --target wasm32-unknown-unknown`
//! (the `net` feature needs sockets, which the browser does not have; the page
//! downloads files itself). The page copies a file into the module's memory,
//! calls [`decode`], then asks for each frame to be drawn with [`render`]:
//! the library scales it to the requested terminal size and picks the 16
//! colours, and the page paints the cells. Exactly the same code as the
//! command line tool.
//!
//! Frames come back as two `u32`s per cell: the character, then
//! `foreground | background << 8` as palette indexes (see [`palette`]).

use std::cell::RefCell;

use termage::{fit, flatten, is_gif, load_frames, load_image, Frame, Options};
use terminal_graphics::Colour;

#[derive(Default)]
struct State {
    /// Bytes the page has written for us to read (a file, or RGBA pixels).
    input: Vec<u8>,
    frames: Vec<Frame>,
    cells: Vec<u32>,
    size: (u32, u32),
    error: String,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

fn palette_index(colour: Colour) -> u32 {
    Colour::ALL.iter().position(|&c| c == colour).unwrap_or(0) as u32
}

impl State {
    /// Draws `image` into `cells` using the library, as `termage` would.
    fn draw(&mut self, image: &image::RgbImage, options: &Options) {
        let display = options.display(image);
        let (cols, rows) = display.size();
        self.size = (cols, rows);
        self.cells.clear();
        for y in 0..rows {
            for x in 0..cols {
                let Some(pixel) = display.pixel(x as isize, y as isize) else {
                    continue;
                };
                self.cells.push(pixel.character() as u32);
                self.cells
                    .push(palette_index(pixel.colour()) | palette_index(pixel.background()) << 8);
            }
        }
    }

    fn decode(&mut self, len: usize) -> u32 {
        self.frames.clear();
        self.error.clear();
        let bytes = &self.input[..len.min(self.input.len())];
        let result = if is_gif(bytes) {
            load_frames(bytes)
        } else {
            load_image(bytes).map(|image| {
                vec![Frame {
                    image,
                    delay: std::time::Duration::ZERO,
                }]
            })
        };
        match result {
            Ok(frames) => {
                self.frames = frames;
                self.frames.len() as u32
            }
            Err(error) => {
                self.error = error.to_string();
                0
            }
        }
    }
}

fn options(cols: u32, rows: u32, dither: u32) -> Options {
    Options::new(cols, rows).with_dither(dither != 0)
}

/// Makes room for `len` bytes of input and returns where to write them.
#[no_mangle]
pub extern "C" fn alloc(len: u32) -> usize {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.input = vec![0; len as usize];
        state.input.as_ptr() as usize
    })
}

/// Decodes the `len` input bytes as an image or gif. Returns the number of
/// frames, or 0 if it could not be decoded (see [`error_ptr`]).
#[no_mangle]
pub extern "C" fn decode(len: u32) -> u32 {
    STATE.with(|state| state.borrow_mut().decode(len as usize))
}

/// How long frame `index` is shown for, in milliseconds (0 for a still image).
#[no_mangle]
pub extern "C" fn frame_delay(index: u32) -> u32 {
    STATE.with(|state| {
        state
            .borrow()
            .frames
            .get(index as usize)
            .map_or(0, |frame| frame.delay.as_millis() as u32)
    })
}

/// Draws decoded frame `index` into a `cols` by `rows` terminal. Returns
/// whether there was such a frame. Read the result with [`cells_ptr`],
/// [`out_cols`] and [`out_rows`].
#[no_mangle]
pub extern "C" fn render(index: u32, cols: u32, rows: u32, dither: u32) -> u32 {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let Some(frame) = state.frames.get(index as usize) else {
            return 0;
        };
        let image = frame.image.clone();
        state.draw(&image, &options(cols, rows, dither));
        1
    })
}

/// Draws the `width` by `height` RGBA pixels the page wrote as input (such as
/// a video frame). Returns whether the input was the right size.
#[no_mangle]
pub extern "C" fn render_rgba(width: u32, height: u32, cols: u32, rows: u32, dither: u32) -> u32 {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let pixels = state.input.clone();
        let Some(rgba) = image::RgbaImage::from_raw(width, height, pixels) else {
            return 0;
        };
        state.draw(&flatten(&rgba), &options(cols, rows, dither));
        1
    })
}

/// The size in pixels a `width` by `height` picture is scaled to in a `cols`
/// by `rows` terminal, as `width | height << 16`.
#[no_mangle]
pub extern "C" fn fit_pack(width: u32, height: u32, cols: u32, rows: u32) -> u32 {
    let (w, h) = fit((width, height), cols, rows);
    w | h << 16
}

/// Columns in the last rendered frame.
#[no_mangle]
pub extern "C" fn out_cols() -> u32 {
    STATE.with(|state| state.borrow().size.0)
}

/// Rows in the last rendered frame.
#[no_mangle]
pub extern "C" fn out_rows() -> u32 {
    STATE.with(|state| state.borrow().size.1)
}

/// Where the last rendered frame's cells are.
#[no_mangle]
pub extern "C" fn cells_ptr() -> usize {
    STATE.with(|state| state.borrow().cells.as_ptr() as usize)
}

/// Where the last error message is, and [`error_len`] how long it is.
#[no_mangle]
pub extern "C" fn error_ptr() -> usize {
    STATE.with(|state| state.borrow().error.as_ptr() as usize)
}

/// The length in bytes of the last error message.
#[no_mangle]
pub extern "C" fn error_len() -> u32 {
    STATE.with(|state| state.borrow().error.len() as u32)
}

/// The `0xRRGGBB` value of palette colour `index` (0 to 15).
#[no_mangle]
pub extern "C" fn palette(index: u32) -> u32 {
    let (r, g, b) = Colour::ALL[index as usize % Colour::ALL.len()].rgb();
    u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageFormat, Rgb, RgbImage};
    use std::io::Cursor;

    fn png(width: u32, height: u32, colour: [u8; 3]) -> Vec<u8> {
        let mut bytes = Vec::new();
        RgbImage::from_pixel(width, height, Rgb(colour))
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .unwrap();
        bytes
    }

    /// What the page does: copy bytes in, then decode.
    fn load(bytes: &[u8]) -> u32 {
        let ptr = alloc(bytes.len() as u32);
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            assert_eq!(ptr, state.input.as_ptr() as usize);
            state.input.copy_from_slice(bytes);
        });
        decode(bytes.len() as u32)
    }

    fn message() -> String {
        STATE.with(|state| state.borrow().error.clone())
    }

    fn cells() -> Vec<u32> {
        STATE.with(|state| state.borrow().cells.clone())
    }

    #[test]
    fn a_png_decodes_to_one_still_frame() {
        assert_eq!(load(&png(8, 8, [255, 0, 0])), 1);
        assert_eq!(frame_delay(0), 0);
    }

    #[test]
    fn junk_does_not_decode_and_says_why() {
        assert_eq!(load(b"not a picture"), 0);
        assert!(message().contains("could not decode"), "{}", message());
        assert_eq!(render(0, 10, 10, 0), 0);
    }

    #[test]
    fn rendering_fills_the_cells_like_the_command_line_would() {
        load(&png(8, 8, [255, 0, 0]));
        assert_eq!(render(0, 4, 2, 0), 1);

        assert_eq!((out_cols(), out_rows()), (4, 2));
        let cells = cells();
        assert_eq!(cells.len(), 4 * 2 * 2);
        let bright_red = palette_index(Colour::BrightRed);
        for cell in cells.chunks(2) {
            assert_eq!(cell[0], '▄' as u32);
            assert_eq!(cell[1], bright_red | bright_red << 8);
        }
    }

    #[test]
    fn dithering_changes_an_in_between_colour() {
        load(&png(16, 16, [106, 106, 106]));
        render(0, 16, 8, 0);
        let plain = cells();
        render(0, 16, 8, 1);
        assert_ne!(plain, cells());
    }

    #[test]
    fn a_missing_frame_is_not_rendered() {
        load(&png(2, 2, [0, 0, 0]));
        assert_eq!(render(5, 4, 2, 0), 0);
    }

    #[test]
    fn rgba_pixels_render_with_transparency_as_black() {
        // 2x2 fully transparent white pixels
        let rgba = [255u8, 255, 255, 0].repeat(4);
        let ptr = alloc(rgba.len() as u32);
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            assert_eq!(ptr, state.input.as_ptr() as usize);
            state.input.copy_from_slice(&rgba);
        });
        assert_eq!(render_rgba(2, 2, 2, 1, 0), 1);
        assert_eq!(cells()[1], 0, "black on black");
    }

    #[test]
    fn rgba_of_the_wrong_length_is_refused() {
        alloc(7);
        assert_eq!(render_rgba(2, 2, 2, 1, 0), 0);
    }

    #[test]
    fn fit_pack_matches_the_library() {
        let packed = fit_pack(100, 100, 80, 10);
        assert_eq!((packed & 0xffff, packed >> 16), (20, 20));
    }

    #[test]
    fn the_palette_is_the_librarys() {
        assert_eq!(palette(0), 0x000000);
        assert_eq!(palette(15), 0xffffff);
        assert_eq!(palette(9), 0xff0000);
    }

    #[test]
    fn the_error_text_is_readable_through_the_exports() {
        load(b"junk");
        assert_eq!(error_len() as usize, message().len());
        assert!(error_len() > 0);
        let _ = error_ptr();
    }

    #[test]
    fn a_gif_has_a_frame_per_picture_with_its_delay() {
        use image::codecs::gif::GifEncoder;
        use image::{Delay, Frame as GifFrame, RgbaImage};

        let mut bytes = Vec::new();
        {
            let mut encoder = GifEncoder::new(&mut bytes);
            for colour in [[255u8, 0, 0, 255], [0, 0, 255, 255]] {
                let image = RgbaImage::from_pixel(4, 4, image::Rgba(colour));
                let frame = GifFrame::from_parts(image, 0, 0, Delay::from_numer_denom_ms(80, 1));
                encoder.encode_frame(frame).unwrap();
            }
        }
        assert_eq!(load(&bytes), 2);
        assert_eq!(frame_delay(1), 80);
        assert_eq!(frame_delay(2), 0);
    }
}
