//! Playing video, by asking `ffmpeg` to decode it.
//!
//! There is no pure Rust video decoder to depend on, so termage runs the
//! `ffmpeg` and `ffprobe` programs (install ffmpeg to use this). ffmpeg
//! scales the video to the size it will have in the terminal and sends raw
//! RGB frames, which are drawn exactly like any other picture. Video is
//! silent.

use std::io::{self, Read, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use image::RgbImage;

use crate::{extension, fit, Error, Options};

/// File extensions that are treated as video.
const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "m4v", "mov", "mkv", "webm", "avi", "mpg", "mpeg", "wmv", "flv", "ogv", "3gp",
];

/// Frames per second used when none is asked for.
pub const DEFAULT_FPS: u32 = 10;

/// Whether `input` (a path or web address) looks like a video, going by its
/// file extension.
///
/// ```
/// assert!(termage::is_video("holiday.MP4"));
/// assert!(termage::is_video("https://example.com/clip.webm?t=3"));
/// assert!(!termage::is_video("cat.gif"));
/// ```
pub fn is_video(input: &str) -> bool {
    extension(input).is_some_and(|extension| VIDEO_EXTENSIONS.contains(&extension.as_str()))
}

/// Plays the video at `input` (a path or web address) in `out`, scaled to fit
/// `options.cols` by `options.rows` cells at `options.fps` frames per second,
/// `options.loops` times (once if `None`).
///
/// Needs `ffmpeg` and `ffprobe` on the `PATH`; without them this returns
/// [`Error::Video`] saying so.
pub fn play_video<W: Write>(out: &mut W, input: &str, options: &Options) -> Result<(), Error> {
    let fps = options.fps.max(1);
    let source = probe(input)?;
    let (width, height) = fit(source, options.cols, options.rows);

    for played in 0..options.loops.unwrap_or(1).max(1) {
        let mut child = Command::new("ffmpeg")
            .args(ffmpeg_args(input, width, height, fps))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(missing_program("ffmpeg"))?;
        let mut stdout = child.stdout.take().expect("stdout is piped");

        let start = Instant::now();
        let mut frame_number = 0u32;
        let mut reserved = played > 0;
        let result = read_frames(&mut stdout, width, height, |frame| {
            let display = options.display(&frame);
            if !reserved {
                out.write_all("\n".repeat(display.height() as usize).as_bytes())?;
                reserved = true;
            }
            display.write_to(out)?;
            out.flush()?;

            // wait until this frame's moment, however long decoding took
            frame_number += 1;
            let due = Duration::from_secs_f64(f64::from(frame_number) / f64::from(fps));
            thread::sleep(due.saturating_sub(start.elapsed()));
            Ok(())
        });
        if result.is_err() {
            // stop ffmpeg decoding for nobody (the pipe was closed)
            let _ = child.kill();
        }
        let _ = child.wait();
        result?;

        if frame_number == 0 {
            let mut message = String::new();
            if let Some(mut stderr) = child.stderr.take() {
                let _ = stderr.read_to_string(&mut message);
            }
            return Err(Error::Video(format!(
                "ffmpeg produced no frames: {}",
                message.trim()
            )));
        }
    }
    Ok(())
}

/// The `(width, height)` of the first video stream in `input`, from `ffprobe`.
fn probe(input: &str) -> Result<(u32, u32), Error> {
    let output = Command::new("ffprobe")
        .args(probe_args(input))
        .stdin(Stdio::null())
        .output()
        .map_err(missing_program("ffprobe"))?;
    if !output.status.success() {
        return Err(Error::Video(format!(
            "ffprobe could not read the video: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    parse_probe(&String::from_utf8_lossy(&output.stdout))
        .ok_or_else(|| Error::Video("the file has no video stream".to_string()))
}

fn missing_program(program: &'static str) -> impl Fn(io::Error) -> Error {
    move |error| {
        if error.kind() == io::ErrorKind::NotFound {
            Error::Video(format!(
                "{program} was not found. Install ffmpeg to play videos"
            ))
        } else {
            Error::Io(error)
        }
    }
}

fn probe_args(input: &str) -> Vec<String> {
    [
        "-v",
        "error",
        "-select_streams",
        "v:0",
        "-show_entries",
        "stream=width,height",
        "-of",
        "csv=s=x:p=0",
        input,
    ]
    .map(String::from)
    .to_vec()
}

/// Parses `ffprobe` output such as `640x360`.
fn parse_probe(output: &str) -> Option<(u32, u32)> {
    let (width, height) = output.lines().next()?.trim().split_once('x')?;
    let (width, height) = (width.parse().ok()?, height.parse().ok()?);
    (width > 0 && height > 0).then_some((width, height))
}

fn ffmpeg_args(input: &str, width: u32, height: u32, fps: u32) -> Vec<String> {
    vec![
        "-v".into(),
        "error".into(),
        "-i".into(),
        input.into(),
        "-an".into(),
        "-vf".into(),
        format!("fps={fps},scale={width}:{height}"),
        "-f".into(),
        "rawvideo".into(),
        "-pix_fmt".into(),
        "rgb24".into(),
        "-".into(),
    ]
}

/// Reads raw RGB frames of `width` by `height` pixels from `reader` and calls
/// `on_frame` for each one, until the stream ends. A frame cut short by the
/// end of the stream is dropped.
fn read_frames<R: Read>(
    reader: &mut R,
    width: u32,
    height: u32,
    mut on_frame: impl FnMut(RgbImage) -> io::Result<()>,
) -> io::Result<()> {
    let mut buffer = vec![0u8; width as usize * height as usize * 3];
    loop {
        let mut filled = 0;
        while filled < buffer.len() {
            match reader.read(&mut buffer[filled..]) {
                Ok(0) => return Ok(()),
                Ok(n) => filled += n,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => return Err(error),
            }
        }
        let frame = RgbImage::from_raw(width, height, buffer.clone())
            .expect("the buffer is exactly width * height * 3 bytes");
        on_frame(frame)?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn video_files_are_recognised_by_extension() {
        for name in ["a.mp4", "A.MKV", "dir/b.webm", "https://x.org/c.mov?x=1"] {
            assert!(is_video(name), "{name}");
        }
        for name in ["a.gif", "a.png", "mp4", "https://x.org/video", ""] {
            assert!(!is_video(name), "{name}");
        }
    }

    #[test]
    fn ffprobe_output_is_parsed() {
        assert_eq!(parse_probe("640x360\n"), Some((640, 360)));
        assert_eq!(parse_probe("1920x1080"), Some((1920, 1080)));
        assert_eq!(parse_probe(""), None);
        assert_eq!(parse_probe("N/A"), None);
        assert_eq!(parse_probe("0x360"), None);
    }

    #[test]
    fn ffmpeg_is_asked_for_scaled_raw_rgb_frames() {
        let args = ffmpeg_args("in.mp4", 80, 40, 12);
        assert!(args.windows(2).any(|w| w == ["-i", "in.mp4"]));
        assert!(args.contains(&"fps=12,scale=80:40".to_string()));
        assert!(args.windows(2).any(|w| w == ["-pix_fmt", "rgb24"]));
        assert_eq!(args.last().unwrap(), "-");
    }

    #[test]
    fn the_input_is_passed_as_one_argument_even_with_spaces() {
        let args = ffmpeg_args("my holiday; rm -rf.mp4", 2, 2, 1);
        assert!(args.contains(&"my holiday; rm -rf.mp4".to_string()));
        assert_eq!(probe_args("a b.mp4").last().unwrap(), "a b.mp4");
    }

    #[test]
    fn frames_are_cut_from_the_stream_and_a_partial_one_is_dropped() {
        // two whole 2x1 frames (6 bytes each) and 4 stray bytes
        let mut stream: Vec<u8> = (0..12).collect();
        stream.extend_from_slice(&[9, 9, 9, 9]);
        let mut frames = Vec::new();
        read_frames(&mut stream.as_slice(), 2, 1, |frame| {
            frames.push(frame);
            Ok(())
        })
        .unwrap();

        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].as_raw(), &(0..6).collect::<Vec<u8>>());
        assert_eq!(frames[1].as_raw(), &(6..12).collect::<Vec<u8>>());
    }

    #[test]
    fn frames_arriving_in_small_pieces_are_reassembled() {
        struct Trickle<'a>(&'a [u8]);
        impl Read for Trickle<'_> {
            fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
                let n = self.0.len().min(buf.len()).min(1);
                buf[..n].copy_from_slice(&self.0[..n]);
                self.0 = &self.0[n..];
                Ok(n)
            }
        }
        let data = [1u8, 2, 3, 4, 5, 6];
        let mut count = 0;
        read_frames(&mut Trickle(&data), 2, 1, |_| {
            count += 1;
            Ok(())
        })
        .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn an_error_from_the_callback_stops_reading() {
        let stream = vec![0u8; 12];
        let mut calls = 0;
        let result = read_frames(&mut stream.as_slice(), 2, 1, |_| {
            calls += 1;
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed"))
        });
        assert!(result.is_err());
        assert_eq!(calls, 1);
    }

    #[test]
    fn a_missing_program_says_to_install_ffmpeg() {
        let error = missing_program("ffmpeg")(io::Error::from(io::ErrorKind::NotFound));
        assert!(error.to_string().contains("Install ffmpeg"));
        let other = missing_program("ffmpeg")(io::Error::from(io::ErrorKind::PermissionDenied));
        assert!(matches!(other, Error::Io(_)));
    }
}
