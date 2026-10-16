[![CI](https://github.com/calum/terminal_image_display/actions/workflows/ci.yml/badge.svg)](https://github.com/calum/terminal_image_display/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/termage.svg)](https://crates.io/crates/termage)
[![docs.rs](https://docs.rs/termage/badge.svg)](https://docs.rs/termage)
[![License](https://img.shields.io/crates/l/termage.svg)](#license)

<div align="center">
  <a href="https://github.com/calum/terminal_image_display">
    <img src="https://raw.githubusercontent.com/calum/terminal_image_display/master/docs/screenshots/termage.png" alt="Termage" height="200" />
  </a>
</div>

<h4 align="center">View images and gifs in your terminal! 🖼️🖥️</h4>

Termage shows PNG, JPEG, GIF, BMP and WebP images in any terminal that
understands ANSI escape codes (Linux, macOS and Windows Terminal / PowerShell).
Animated gifs play. It is built on
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

Display any image or animated gif in the terminal with Termage!

Usage: termage [OPTIONS] <FILE>

Arguments:
  <FILE>  The image to show: PNG, JPEG, GIF, BMP or WebP. Animated gifs play

Options:
  -w, --width <WIDTH>    Width in columns [default: the width of the terminal]
  -H, --height <HEIGHT>  Height in rows [default: the height of the terminal]
  -l, --loops <N>        Play an animated gif this many times [default: until stopped]
  -h, --help             Print help
  -V, --version          Print version
```

The picture is scaled to fit, keeping its proportions. Transparent pixels are
shown as black. Gifs are recognised by their contents, not their file name.
Stop a looping gif with Ctrl-C.

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
use std::path::Path;

fn main() -> Result<(), termage::Error> {
    // show a picture (or play a gif) in an 80 column by 24 row area
    termage::show(&mut std::io::stdout(), Path::new("ferris.png"), 80, 24, None)
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
