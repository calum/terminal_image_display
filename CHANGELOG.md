# Changelog

## 4.0.0

A website with a terminal you can try in the browser, dithering, and sharper
screenshots. The library API changed, so this is a major version (the command
line only gains options).

### Breaking changes (library)

`Options` now carries every setting (including the new `dither`), so the
functions that took loose size and loop arguments take it instead. `Options` is
`#[non_exhaustive]`, so more settings can be added later without another major
version: build it with `Options::new` and the `with_*` methods.

| 3.0 | 4.0 |
| --- | --- |
| `show_image(&mut out, &image, cols, rows)` | `show_image(&mut out, &image, &Options::new(cols, rows))` |
| `play(&mut out, &frames, cols, rows, loops)` | `play(&mut out, &frames, &Options::new(cols, rows).with_loops(loops))` |
| `play_video(&mut out, input, cols, rows, fps, loops)` | `play_video(&mut out, input, &Options::new(cols, rows).with_fps(fps).with_loops(Some(loops)))` |
| `Options { cols, rows, loops, fps }` (struct literal) | `Options::new(cols, rows).with_loops(loops).with_fps(fps)` |

### Added

- `--dither` / `-d`: Floyd-Steinberg dithering, so gradients, skies and photos
  look far better with only 16 colours. Also `Options::with_dither` and
  `to_display_dithered`.
- A website, <https://calum.github.io/terminal_image_display/>, with a terminal
  running the real library as WebAssembly: example commands, and drag and drop
  for your own images, gifs and video. It is generated from Rust
  (`examples/build_site.rs`, `examples/web.rs`) and deployed by
  `.github/workflows/pages.yml`.
- A `net` cargo feature (on by default) for loading from web addresses. Build
  with `default-features = false` to drop the HTTP client, which is needed to
  compile for WebAssembly.
- `flatten` and `Options::display` in the library.
- `examples/make_samples.rs` draws the sample pictures in `docs/samples`
  (a transparent PNG, a JPEG, a detailed PNG and two animated gifs).
  `examples/render_gif.rs` can now save a png, and takes `--dither`.
- A repo hygiene test: only Rust and a short list of config and image files are
  committed.

### Changed

- The README logo and example screenshots were regenerated from scratch at a
  much higher resolution, and show off transparency, JPEG, fine detail,
  dithering and animation.

## 3.0.0

Web addresses, video and a gif logo. The library functions now work on bytes and
text inputs so they can read from more than files, which breaks the 2.0 library
API (the command line is unchanged, apart from what is added).

### Breaking changes (library)

| 2.0 | 3.0 |
| --- | --- |
| `show(&mut out, path: &Path, cols, rows, loops)` | `show(&mut out, input: &str, &Options { loops, ..Options::new(cols, rows) })`; `input` is a path or an http(s) address |
| `load_image(path: &Path)` | `load_image(&read(input)?)` (decodes bytes) |
| `load_frames(path: &Path)` | `load_frames(&read(input)?)` (decodes bytes) |
| `is_gif(path: &Path) -> Result<bool, Error>` | `is_gif(&bytes) -> bool` |
| `Error` has `Io` and `Image` | also `Download` and `Video`: match with a wildcard arm |

### Added

- Images and gifs can be loaded from web addresses:
  `termage https://example.com/cat.gif`. Downloads over 100 MB are refused.
- Video playback: `termage clip.mp4` (mp4, mkv, webm, mov, avi and more), using
  `ffmpeg` if it is installed, with `--fps` and `--loops`. Silent, and plays
  once unless `--loops` is given.
- `examples/render_gif.rs`, which renders a picture the way termage shows it and
  saves a gif. The README logo is now a gif made with it.
- `Options`, `play_video`, `is_url`, `is_video`, `read` and `extension` in the
  library.

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
| `display_image(path, w, h)` | `show(&mut stdout, path, w, h, None)?` (also plays gifs) |
| `display_gif(path, w, h)` | `show(&mut stdout, path, w, h, loops)?` |
| `get_image(path) -> RgbImage` (panicked) | `load_image(path) -> Result<RgbImage, Error>` |
| `get_gif(path) -> Frames` (panicked) | `load_frames(path) -> Result<Vec<Frame>, Error>` |
| `pixelate_image`, `closest_match`, `merge_pixels` | removed: scaling uses the `image` crate's filters (`to_display`) |
| `image` 0.19, `terminal_graphics` 0.1, `clap` 2 | `image` 0.25, `terminal_graphics` 1, `clap` 4 |

### Added

- `--width`, `--height` and `--loops` options, and `--version` from Cargo.
- `show`, `show_image`, `play`, `fit`, `to_display`, `pick_size`, `is_gif`,
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
