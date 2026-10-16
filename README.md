[![CI](https://github.com/calum/terminal_image_display/actions/workflows/ci.yml/badge.svg)](https://github.com/calum/terminal_image_display/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/termage.svg)](https://crates.io/crates/termage)
[![docs.rs](https://docs.rs/termage/badge.svg)](https://docs.rs/termage)
[![License](https://img.shields.io/crates/l/termage.svg)](#license)

<div align="center">
  <a href="https://github.com/calum/terminal_image_display">
    <img src="https://raw.githubusercontent.com/calum/terminal_image_display/master/docs/screenshots/termage.gif" alt="Termage" height="200" />
  </a>
</div>

<h4 align="center">View images and gifs in your terminal! 🖼️🖥️</h4>

Termage shows PNG, JPEG, GIF, BMP and WebP images, plays animated gifs and
(with ffmpeg) videos, from files or web addresses, in any terminal that
understands ANSI escape codes (Linux, macOS and Windows Terminal / PowerShell).
It is built on
[terminal_graphics](https://crates.io/crates/terminal_graphics).

## Install

You can install using [Cargo](https://crates.io/)
```text
cargo install termage
```

or you can clone the repo and build the binary
```text
git clone https://github.com/calum/terminal_image_display
cd terminal_image_display
cargo run -- path/to/ferris.png
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
  -h, --help             Print help
  -V, --version          Print version
```

The picture is scaled to fit, keeping its proportions. Transparent pixels are
shown as black. Gifs are recognised by their contents, not their file name.
Stop a looping gif with Ctrl-C.

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
```text
termage ferris.png
```
![](https://raw.githubusercontent.com/calum/terminal_image_display/master/docs/screenshots/ferris_termage.png)

```text
termage pika.gif
```
![](https://raw.githubusercontent.com/calum/terminal_image_display/master/docs/screenshots/pika_termage.gif)

```text
termage pika.png
```
![](https://raw.githubusercontent.com/calum/terminal_image_display/master/docs/screenshots/pika_termage.png)

## Using it as a library

```toml
[dependencies]
termage = "2"
```

```rust,no_run
use termage::Options;

fn main() -> Result<(), termage::Error> {
    // show a picture, play a gif or a video (needs ffmpeg); a file or a web
    // address, in an 80 column by 24 row area
    termage::show(&mut std::io::stdout(), "ferris.png", &Options::new(80, 24))
}
```

The [API documentation](https://docs.rs/termage) covers loading, scaling
(`fit`, `to_display`) and playing frames yourself. See the
[CHANGELOG](CHANGELOG.md) for what changed in 2.0 if you are upgrading.

## Links
- crates.io: https://crates.io/crates/termage
- API documentation: https://docs.rs/termage
- Source: https://github.com/calum/terminal_image_display
- Issues: https://github.com/calum/terminal_image_display/issues

## License

Licensed under either of MIT or Apache-2.0 at your option.
