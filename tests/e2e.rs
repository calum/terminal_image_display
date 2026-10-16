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
    assert!(text.contains("--width") && text.contains("--loops") && text.contains("INPUT"));

    let version = termage(&["--version"]);
    assert!(stdout(&version).contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn running_without_a_file_explains_how_to_use_it() {
    let output = termage(&[]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("Usage"));
}

#[cfg(feature = "net")]
/// Serves `body` once over HTTP on a free local port and returns the address.
fn serve_once(status: &'static str, body: Vec<u8>, content_type: &'static str) -> String {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0u8; 2048];
        let _ = stream.read(&mut request);
        let head = format!(
            "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        let _ = stream.write_all(head.as_bytes());
        let _ = stream.write_all(&body);
    });
    format!("http://{address}")
}

#[cfg(feature = "net")]
#[test]
fn shows_an_image_from_a_web_address() {
    let dir = TempDir::new("url");
    let png = dir.path("blue.png");
    RgbImage::from_pixel(8, 8, Rgb([0, 0, 255]))
        .save(&png)
        .unwrap();
    let url = serve_once("200 OK", fs::read(&png).unwrap(), "image/png");

    let output = termage(&[&format!("{url}/blue.png"), "-w", "4", "-H", "2"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output).matches('▄').count(), 4 * 2);
    assert!(stdout(&output).contains("\x1b[94;104m▄"));
}

#[cfg(feature = "net")]
#[test]
fn plays_a_gif_from_a_web_address() {
    let dir = TempDir::new("urlgif");
    let gif = dir.path("blink.gif");
    write_gif(&gif, &[[255, 0, 0, 255], [0, 0, 255, 255]]);
    let url = serve_once("200 OK", fs::read(&gif).unwrap(), "image/gif");

    let output = termage(&[&format!("{url}/blink.gif"), "-w", "2", "-H", "1", "-l", "1"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output).matches('▄').count(), 2 * 2);
}

#[cfg(feature = "net")]
#[test]
fn a_web_address_that_fails_is_a_clean_error() {
    let url = serve_once("404 Not Found", b"nope".to_vec(), "text/plain");
    let output = termage(&[&format!("{url}/missing.png")]);
    assert!(!output.status.success());
    let message = stderr(&output);
    assert!(message.contains("could not download"), "{message}");
    assert!(message.contains("404"), "{message}");
    assert!(!message.contains("panicked"));
}

#[cfg(feature = "net")]
#[test]
fn a_web_page_that_is_not_an_image_is_a_clean_error() {
    let url = serve_once("200 OK", b"<html>hi</html>".to_vec(), "text/html");
    let output = termage(&[&format!("{url}/page")]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("could not decode the image"));
}

#[test]
fn playing_a_video_without_ffmpeg_says_how_to_fix_it() {
    let dir = TempDir::new("noffmpeg");
    let output = Command::new(env!("CARGO_BIN_EXE_termage"))
        .arg("clip.mp4")
        // an empty directory: neither ffmpeg nor ffprobe can be found
        .env("PATH", dir.path(""))
        .output()
        .unwrap();
    assert!(!output.status.success());
    let message = stderr(&output);
    assert!(message.contains("Install ffmpeg"), "{message}");
    assert!(!message.contains("panicked"));
}

fn has_ffmpeg() -> bool {
    ["ffmpeg", "ffprobe"].iter().all(|program| {
        Command::new(program)
            .arg("-version")
            .output()
            .is_ok_and(|output| output.status.success())
    })
}

/// Makes a one second test pattern video, or `None` if ffmpeg can't.
fn make_video(path: &Path) -> bool {
    Command::new("ffmpeg")
        .args(["-v", "error", "-y", "-f", "lavfi", "-i"])
        .arg("testsrc=duration=1:size=64x48:rate=10")
        .args(["-c:v", "mjpeg"])
        .arg(path)
        .status()
        .is_ok_and(|status| status.success())
}

#[test]
fn plays_a_video_when_ffmpeg_is_installed() {
    let dir = TempDir::new("video");
    let video = dir.path("test.avi");
    if !has_ffmpeg() || !make_video(&video) {
        eprintln!("skipped: ffmpeg is not available");
        return;
    }

    let output = termage(&[video.to_str().unwrap(), "-w", "16", "-H", "6", "--fps", "5"]);
    assert!(output.status.success(), "{}", stderr(&output));

    // 64x48 in a 16x6 area (12 pixel rows) is 16x12 pixels: 16 columns by 6 rows.
    // One second at 5 fps is 5 frames.
    let cells = 16 * 6;
    let count = stdout(&output).matches('▄').count();
    assert!(
        count >= cells * 4 && count <= cells * 6,
        "{count} cells drawn"
    );
}

#[test]
fn dithering_mixes_colours_for_in_between_shades() {
    let dir = TempDir::new("dither");
    let png = dir.path("grey.png");
    RgbImage::from_pixel(16, 16, Rgb([106, 106, 106]))
        .save(&png)
        .unwrap();
    let path = png.to_str().unwrap();

    let plain = stdout(&termage(&[path, "-w", "16", "-H", "8"]));
    let dithered = stdout(&termage(&[path, "-w", "16", "-H", "8", "--dither"]));

    let colours = |text: &str| {
        let mut codes: Vec<&str> = text.split('\x1b').filter(|s| s.ends_with("▄")).collect();
        codes.sort();
        codes.dedup();
        codes.len()
    };
    assert_eq!(colours(&plain), 1);
    assert!(colours(&dithered) > 1);
}

#[cfg(feature = "net")]
/// A stand-in for the GitHub release pages: `/latest` says which version is
/// newest, and anything under `/download/` is the new program.
fn serve_releases(latest: &'static str, program: &'static [u8]) -> String {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { return };
            let mut request = [0u8; 2048];
            let read = stream.read(&mut request).unwrap_or(0);
            let request = String::from_utf8_lossy(&request[..read]).to_string();
            let body = if request.starts_with("GET /latest") {
                format!("{{\"tag_name\": \"{latest}\"}}").into_bytes()
            } else if request.starts_with("GET /download/") {
                program.to_vec()
            } else {
                Vec::new()
            };
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(&body);
        }
    });
    format!("http://{address}")
}

#[cfg(feature = "net")]
/// Runs a copy of termage (so updating cannot damage the one under test).
fn termage_copy(dir: &TempDir, base: &str, args: &[&str]) -> (PathBuf, Output) {
    let copy = dir.path(if cfg!(windows) {
        "termage.exe"
    } else {
        "termage"
    });
    fs::copy(env!("CARGO_BIN_EXE_termage"), &copy).unwrap();
    let output = Command::new(&copy)
        .args(args)
        .env("TERMAGE_UPDATE_BASE", base)
        .output()
        .expect("failed to run termage");
    (copy, output)
}

#[cfg(feature = "net")]
#[test]
fn update_replaces_the_program_with_the_newest_release() {
    let dir = TempDir::new("update");
    let base = serve_releases("v99.0.0", b"the new termage");

    let (copy, output) = termage_copy(&dir, &base, &["--update"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("to 99.0.0"), "{}", stdout(&output));
    assert_eq!(fs::read(&copy).unwrap(), b"the new termage");
    // Windows cannot delete the program while it is still running, so the old
    // copy stays as `termage.old` until the next update; elsewhere nothing is left
    let leftovers = if cfg!(windows) { 2 } else { 1 };
    assert_eq!(fs::read_dir(&dir.0).unwrap().count(), leftovers);
}

#[cfg(feature = "net")]
#[test]
fn update_leaves_the_program_alone_when_it_is_already_the_newest() {
    let dir = TempDir::new("current");
    let base = serve_releases("v0.0.1", b"an older termage");

    let (copy, output) = termage_copy(&dir, &base, &["--update"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("already up to date"));
    assert_eq!(
        fs::read(&copy).unwrap(),
        fs::read(env!("CARGO_BIN_EXE_termage")).unwrap()
    );
}

#[cfg(feature = "net")]
#[test]
fn update_reports_a_failure_in_one_line() {
    let dir = TempDir::new("offline");

    // port 1 on localhost: nothing listens there
    let (copy, output) = termage_copy(&dir, "http://127.0.0.1:1", &["--update"]);

    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).starts_with("termage: update failed:"));
    assert_eq!(
        fs::read(&copy).unwrap(),
        fs::read(env!("CARGO_BIN_EXE_termage")).unwrap()
    );
}
