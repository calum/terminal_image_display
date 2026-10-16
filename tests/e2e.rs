//! End to end: run the real `termage` binary on real files and check what it
//! prints and how it exits.

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use image::codecs::gif::GifEncoder;
use image::{Delay, Frame, Rgb, RgbImage, Rgba, RgbaImage};

/// A fresh temp directory for one test, removed when dropped.
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> TempDir {
        let dir = std::env::temp_dir().join(format!("termage_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }

    fn path(&self, file: &str) -> PathBuf {
        self.0.join(file)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn termage(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_termage"))
        .args(args)
        .output()
        .expect("failed to run termage")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

fn write_png(path: &Path, width: u32, height: u32) {
    RgbImage::from_pixel(width, height, Rgb([255, 0, 0]))
        .save(path)
        .unwrap();
}

fn write_gif(path: &Path, colours: &[[u8; 4]]) {
    let mut encoder = GifEncoder::new(File::create(path).unwrap());
    for colour in colours {
        let image = RgbaImage::from_pixel(4, 4, Rgba(*colour));
        let frame = Frame::from_parts(image, 0, 0, Delay::from_numer_denom_ms(20, 1));
        encoder.encode_frame(frame).unwrap();
    }
}

#[test]
fn shows_a_png_scaled_to_the_requested_size() {
    let dir = TempDir::new("png");
    let png = dir.path("red.png");
    write_png(&png, 40, 20);

    // 40x20 fits 8 columns as 8x4 pixels, which is 8 columns by 2 rows
    let output = termage(&[png.to_str().unwrap(), "--width", "8", "--height", "10"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stderr(&output).is_empty());

    let text = stdout(&output);
    assert_eq!(text.matches('▄').count(), 8 * 2);
    // bright red is the closest of the 16 colours
    assert!(text.contains("\x1b[91;101m▄"));
    // room is reserved for the picture, then the cursor ends below it
    assert!(text.starts_with("\n\n\x1b[2F"));
}

#[test]
fn plays_a_gif_the_requested_number_of_loops() {
    let dir = TempDir::new("gif");
    let gif = dir.path("blink.gif");
    write_gif(&gif, &[[255, 0, 0, 255], [0, 0, 255, 255]]);

    let output = termage(&[gif.to_str().unwrap(), "-w", "4", "-H", "2", "--loops", "2"]);
    assert!(output.status.success(), "{}", stderr(&output));

    // 4x4 pixels in a 4x2 area: 4 columns by 2 rows = 8 cells, 2 frames, 2 loops
    assert_eq!(stdout(&output).matches('▄').count(), 8 * 2 * 2);
    let text = stdout(&output);
    assert!(text.contains("\x1b[91;101m"), "first frame is red");
    assert!(text.contains("\x1b[94;104m"), "second frame is blue");
}

#[test]
fn an_animated_gif_is_detected_by_content_not_by_name() {
    let dir = TempDir::new("sniff");
    let disguised = dir.path("picture.png");
    write_gif(&disguised, &[[0, 255, 0, 255], [0, 255, 0, 255]]);

    let output = termage(&[disguised.to_str().unwrap(), "-w", "2", "-H", "1", "-l", "1"]);
    assert!(output.status.success(), "{}", stderr(&output));
    // two frames, one loop, one cell wide pair each
    assert_eq!(stdout(&output).matches('▄').count(), 2 * 2);
}

#[test]
fn a_missing_file_is_a_clean_error() {
    let output = termage(&["no/such/file.png"]);
    assert!(!output.status.success());
    assert!(stdout(&output).is_empty());
    let message = stderr(&output);
    assert!(
        message.starts_with("termage: no/such/file.png: "),
        "{message}"
    );
    assert!(!message.contains("panicked"));
}

#[test]
fn a_file_that_is_not_an_image_is_a_clean_error() {
    let dir = TempDir::new("text");
    let text = dir.path("notes.png");
    fs::write(&text, "this is not a picture").unwrap();

    let output = termage(&[text.to_str().unwrap()]);
    assert!(!output.status.success());
    let message = stderr(&output);
    assert!(message.contains("could not decode the image"), "{message}");
    assert!(!message.contains("panicked"));
}

#[test]
fn transparent_png_pixels_are_shown_as_black() {
    let dir = TempDir::new("alpha");
    let png = dir.path("clear.png");
    RgbaImage::from_pixel(2, 2, Rgba([255, 255, 255, 0]))
        .save(&png)
        .unwrap();

    let output = termage(&[png.to_str().unwrap(), "-w", "2", "-H", "1"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("\x1b[30;40m▄"));
}

#[test]
fn help_and_version_work() {
    let help = termage(&["--help"]);
    assert!(help.status.success());
    let text = stdout(&help);
    assert!(text.contains("--width") && text.contains("--loops") && text.contains("FILE"));

    let version = termage(&["--version"]);
    assert!(stdout(&version).contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn running_without_a_file_explains_how_to_use_it() {
    let output = termage(&[]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("Usage"));
}
