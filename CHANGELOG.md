# Changelog

## 2.0.0

Updated to `terminal_graphics` 1.0, and rewritten around a small, tested library
with proper error handling. The command line changed, so this is a major
release.

### Breaking changes

**Command line.** The file is now a positional argument and the `-i` / `-g`
flags are gone. A gif is recognised by its contents, so one command handles both.

| 1.x | 2.0 |
| --- | --- |
| `termage -i ferris.png` | `termage ferris.png` |
| `termage -g pika.gif` | `termage pika.gif` |
| (always the full terminal) | `--width` / `-w` and `--height` / `-H` set the size |
| (gifs looped forever) | still the default, or `--loops N` / `-l N` to stop after N plays |

Errors (missing file, not an image) now print a one-line message to stderr and
exit with status 1, instead of panicking.

**Library.** The library API was rewritten:

| 1.x | 2.0 |
| --- | --- |
| `display_image(path, w, h)` | `show(&mut stdout, path, &Options::new(w, h))?` (also plays gifs and videos) |
| `display_gif(path, w, h)` | `show(&mut stdout, path, &Options { loops, ..Options::new(w, h) })?` |
| `get_image(path) -> RgbImage` (panicked) | `load_image(&read(path)?)` (decodes bytes, returns a `Result`) |
| `get_gif(path) -> Frames` (panicked) | `load_frames(&read(path)?)` (returns `Result<Vec<Frame>, Error>`) |
| `pixelate_image`, `closest_match`, `merge_pixels` | removed: scaling uses the `image` crate's filters (`to_display`) |
| `image` 0.19, `terminal_graphics` 0.1, `clap` 2 | `image` 0.25, `terminal_graphics` 1, `clap` 4 |

### Added

- Images and gifs can be loaded from web addresses: `termage https://example.com/cat.gif`.
- Video playback: `termage clip.mp4` (mp4, mkv, webm, mov, avi and more), using
  `ffmpeg` if it is installed, with `--fps` and `--loops`. Silent.
- `examples/render_gif.rs`, which renders a picture the way termage shows it and
  saves a gif. The README logo is now a gif made with it.
- `--width`, `--height`, `--loops` and `--fps` options, and `--version` from Cargo.
- `show`, `Options`, `show_image`, `play`, `play_video`, `fit`, `to_display`,
  `pick_size`, `is_gif`, `is_url`, `is_video`, `read`, `extension`,
  `Frame` and `Error` in the library.
- WebP and BMP support (and PNG, JPEG and GIF as before).
- Tests (unit, integration and end to end), CI, a docs build, and automatic
  publishing when the version is bumped.

### Changed

- Images are scaled with a proper averaging filter (nearest neighbour when
  scaling up, so pixel art stays sharp). The old averaging code dimmed colours.
- Transparent pixels are drawn as black.
- Gif frame delays are read correctly (milliseconds), with browser-style
  handling of very short delays. The gif is decoded once and replayed from
  memory.
- Space is reserved before drawing, so the picture no longer overwrites the
  lines above it.
- The size falls back to 80x24 when the terminal size is unknown, instead of
  refusing to run, and one row is left for the shell prompt.
