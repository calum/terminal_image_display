[![CI](https://github.com/calum/terminal_image_display/actions/workflows/ci.yml/badge.svg)](https://github.com/calum/terminal_image_display/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/termage.svg)](https://crates.io/crates/termage)
[![docs.rs](https://docs.rs/termage/badge.svg)](https://docs.rs/termage)
[![License](https://img.shields.io/crates/l/termage.svg)](#license)

<div align="center">
  <a href="https://calum.github.io/terminal_image_display/">
    <img src="https://raw.githubusercontent.com/calum/terminal_image_display/master/docs/screenshots/termage.gif" alt="Termage" height="200" />
  </a>
</div>

<h4 align="center">View images, gifs and video in your terminal! 🖼️🖥️</h4>

**[Try it in your browser: run example commands or drop in your own pictures](https://calum.github.io/terminal_image_display/)**

Termage shows PNG, JPEG, GIF, BMP and WebP images, plays animated gifs and
(with ffmpeg) videos, from files or web addresses, in any terminal that
understands ANSI escape codes (Linux, macOS and Windows Terminal / PowerShell).
It is built on [terminal_graphics](https://crates.io/crates/terminal_graphics).

## Install

You can install using [Cargo](https://crates.io/)
```text
cargo install termage
```

or you can clone the repo and build the binary
```text
git clone https://github.com/calum/terminal_image_display
cd terminal_image_display
cargo run -- docs/samples/crab.png
```

## Usage
```text
$ termage --help

Display any image, animated gif or video in the terminal with Termage!

Usage: termage [OPTIONS] <INPUT>

Arguments:
  <INPUT>  The picture to show: a file or an http(s) address. PNG, JPEG, GIF, BMP and WebP images work, and animated gifs play. Videos (mp4, mkv, webm, ...) play silently if ffmpeg is installed

Options:
  -w, --width <WIDTH>    Width in columns [default: the width of the terminal]
  -H, --height <HEIGHT>  Height in rows [default: the height of the terminal]
  -l, --loops <N>        Play a gif or video this many times [default: gifs until stopped, videos once]
      --fps <N>          Frames per second for video [default: 10]
  -d, --dither           Dither to make gradients and photos look smoother with only 16 colours
  -h, --help             Print help
  -V, --version          Print version
```

The picture is scaled to fit, keeping its proportions. Every terminal cell
shows two pixels, one above the other, using a half block character. Transparent
pixels are shown as black. Gifs are recognised by their contents, not their file
name. Stop a looping gif with Ctrl-C.

Terminals only have 16 colours to work with, so photos and gradients look flat.
`--dither` mixes colours cell by cell to fake the ones in between:

```text
termage docs/samples/sunset.jpg --dither
```

### From a web address

```text
termage https://example.com/cat.png
termage https://example.com/dance.gif
```

### Video

```text
termage holiday.mp4 --fps 12
```

Video needs [ffmpeg](https://ffmpeg.org/download.html) (`ffmpeg` and `ffprobe`)
on your `PATH`; termage asks it to decode the frames. It is silent, and plays
once unless you pass `--loops`.

## Example output

These are made from the pictures in [`docs/samples`](docs/samples), which are
drawn by code (`cargo run --example make_samples`). The screenshots show exactly
what the terminal shows, and are made with `cargo run --example render_gif`.

A transparent PNG, with `--dither`:
```text
termage docs/samples/crab.png --dither
```
![](https://raw.githubusercontent.com/calum/terminal_image_display/master/docs/screenshots/crab.png)

A JPEG, with `--dither`:
```text
termage docs/samples/sunset.jpg --dither
```
![](https://raw.githubusercontent.com/calum/terminal_image_display/master/docs/screenshots/sunset.png)

Fine detail:
```text
termage docs/samples/mandelbrot.png --dither
```
![](https://raw.githubusercontent.com/calum/terminal_image_display/master/docs/screenshots/mandelbrot.png)

Animated gifs:
```text
termage docs/samples/plasma.gif --dither
termage docs/samples/bounce.gif --dither
```
![](https://raw.githubusercontent.com/calum/terminal_image_display/master/docs/screenshots/plasma.gif)
![](https://raw.githubusercontent.com/calum/terminal_image_display/master/docs/screenshots/bounce.gif)

## Using it as a library

```toml
[dependencies]
termage = "4"
```

```rust,no_run
use termage::Options;

fn main() -> Result<(), termage::Error> {
    // show a picture, play a gif or a video (needs ffmpeg); a file or a web
    // address, in an 80 column by 24 row area
    let options = Options::new(80, 24).with_dither(true);
    termage::show(&mut std::io::stdout(), "crab.png", &options)
}
```

The [API documentation](https://docs.rs/termage) covers loading, scaling
(`fit`, `to_display`) and playing frames yourself. Turn off the default `net`
feature (`default-features = false`) to drop the HTTP client, for example to
compile for WebAssembly. See the [CHANGELOG](CHANGELOG.md) for what changed in
each major version if you are upgrading.

## Links
- Website (with a playable terminal): https://calum.github.io/terminal_image_display/
- crates.io: https://crates.io/crates/termage
- API documentation: https://docs.rs/termage
- Source: https://github.com/calum/terminal_image_display
- Issues: https://github.com/calum/terminal_image_display/issues

## License

Licensed under either of MIT or Apache-2.0 at your option.
