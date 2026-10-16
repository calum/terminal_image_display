//! Builds the project website into a directory (default `target/site`).
//!
//! ```text
//! cargo build --release --example web --no-default-features --target wasm32-unknown-unknown
//! cargo run --example build_site -- [output-dir]
//! ```
//!
//! The page is generated from Rust so the repository stays Rust only: the
//! colours come straight from `terminal_graphics::Colour`, the sample pictures
//! in `docs/samples` are copied alongside, and the compiled `web` example (the
//! termage library, as WebAssembly) is copied in as `termage.wasm`. The page
//! is deployed to GitHub Pages by `.github/workflows/pages.yml`.

use std::fs;
use std::path::{Path, PathBuf};

use terminal_graphics::Colour;

const SITE_URL: &str = "https://calum.github.io/terminal_image_display/";
const REPO_URL: &str = "https://github.com/calum/terminal_image_display";
const REPO_PATH: &str = "calum/terminal_image_display";
const WASM_PATH: &str = "target/wasm32-unknown-unknown/release/examples/web.wasm";
const SAMPLES_DIR: &str = "docs/samples";

/// Commands offered as buttons under the terminal: command, then what it shows.
const EXAMPLES: &[(&str, &str)] = &[
    ("termage crab.png", "a transparent PNG"),
    ("termage sunset.jpg --dither", "a JPEG, dithered"),
    ("termage mandelbrot.png -w 60", "fine detail, scaled down"),
    ("termage plasma.gif --dither", "an animated gif"),
    ("termage bounce.gif -w 48", "a smaller gif"),
    ("termage termage.gif", "the logo"),
    ("termage --help", "every option"),
];

/// `curl -fsSL <site>/install.sh | sh` for Linux and macOS: downloads the right
/// release file and puts it in `~/.local/bin` (or `$TERMAGE_INSTALL_DIR`).
const INSTALL_SH: &str = r####"#!/bin/sh
# Installs termage from the latest GitHub release.
#   curl -fsSL {{SITE_URL}}install.sh | sh
# TERMAGE_INSTALL_DIR chooses the folder (default ~/.local/bin) and
# TERMAGE_VERSION a release other than the latest (for example 4.1.0).
set -eu

repo="{{REPO}}"
dir="${TERMAGE_INSTALL_DIR:-$HOME/.local/bin}"

case "$(uname -s)" in
  Linux) os=unknown-linux-gnu ;;
  Darwin) os=apple-darwin ;;
  *) echo "termage: no release for $(uname -s); try: cargo install termage" >&2; exit 1 ;;
esac
case "$(uname -m)" in
  x86_64 | amd64) arch=x86_64 ;;
  aarch64 | arm64) arch=aarch64 ;;
  *) echo "termage: no release for $(uname -m); try: cargo install termage" >&2; exit 1 ;;
esac

file="termage-$arch-$os"
if [ -n "${TERMAGE_VERSION:-}" ]; then
  url="https://github.com/$repo/releases/download/v${TERMAGE_VERSION#v}/$file"
else
  url="https://github.com/$repo/releases/latest/download/$file"
fi

mkdir -p "$dir"
tmp="$dir/.termage.download"
echo "Downloading $url"
if command -v curl >/dev/null 2>&1; then
  curl -fsSL "$url" -o "$tmp"
elif command -v wget >/dev/null 2>&1; then
  wget -qO "$tmp" "$url"
else
  echo "termage: curl or wget is needed to download" >&2
  exit 1
fi
chmod +x "$tmp"
mv -f "$tmp" "$dir/termage"

echo "Installed termage to $dir/termage"
case ":$PATH:" in
  *":$dir:"*) ;;
  *) echo "Add it to your PATH:  export PATH=\"$dir:\$PATH\"" ;;
esac
"####;

/// `irm <site>/install.ps1 | iex` for Windows PowerShell: installs to
/// `%LOCALAPPDATA%\Programs\termage` and adds that folder to the user's PATH.
const INSTALL_PS1: &str = r####"# Installs termage from the latest GitHub release.
#   irm {{SITE_URL}}install.ps1 | iex
# $env:TERMAGE_INSTALL_DIR chooses the folder and $env:TERMAGE_VERSION a
# release other than the latest (for example 4.1.0).
$ErrorActionPreference = 'Stop'

$repo = '{{REPO}}'
$dir = if ($env:TERMAGE_INSTALL_DIR) { $env:TERMAGE_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'Programs\termage' }

if ($env:PROCESSOR_ARCHITECTURE -ne 'AMD64') {
    throw "termage: no release for $env:PROCESSOR_ARCHITECTURE; try: cargo install termage"
}
$file = 'termage-x86_64-pc-windows-msvc.exe'
if ($env:TERMAGE_VERSION) {
    $url = "https://github.com/$repo/releases/download/v$($env:TERMAGE_VERSION.TrimStart('v'))/$file"
} else {
    $url = "https://github.com/$repo/releases/latest/download/$file"
}

New-Item -ItemType Directory -Force -Path $dir | Out-Null
Write-Host "Downloading $url"
Invoke-WebRequest -Uri $url -OutFile (Join-Path $dir 'termage.exe') -UseBasicParsing
Write-Host "Installed termage to $dir\termage.exe"

$user = [Environment]::GetEnvironmentVariable('Path', 'User')
if (($user -split ';') -notcontains $dir) {
    [Environment]::SetEnvironmentVariable('Path', "$user;$dir", 'User')
    Write-Host 'Added it to your PATH. Open a new terminal to use termage.'
}
"####;

/// The install scripts with the repo and site addresses filled in.
fn install_scripts() -> [(&'static str, String); 2] {
    let fill = |script: &str| {
        script
            .replace("{{SITE_URL}}", SITE_URL)
            .replace("{{REPO}}", REPO_PATH)
    };
    [
        ("install.sh", fill(INSTALL_SH)),
        ("install.ps1", fill(INSTALL_PS1)),
    ]
}

const PAGE: &str = r####"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>termage - view images, gifs and video in your terminal</title>
<meta name="description" content="Termage shows images, animated gifs and videos in the terminal with 16 ANSI colours. Try it in your browser: run example commands or drop in your own pictures.">
<link rel="canonical" href="{{SITE_URL}}">
<link rel="icon" href="data:,">
<style>
{{CSS}}
</style>
</head>
<body>
<div class="crt">
<main>

<p class="dim">TERMAGE BBS v{{VERSION}} - CONNECTED 14400 BAUD</p>
<h1 aria-label="TERMAGE">{{TITLE}}</h1>
<p class="subtitle">images | gifs | video | 16 ANSI colours | 100% Rust</p>
<p class="nav">
<a href="{{REPO_URL}}">[ GitHub ]</a>
<a href="https://crates.io/crates/termage">[ crates.io ]</a>
<a href="https://docs.rs/termage">[ docs.rs ]</a>
</p>

<p class="nav"><a href="#install">[ Install ]</a> <a href="#try">[ Try it ]</a></p>

<section id="install">
<h2>&gt; INSTALL</h2>
<p>Pick one. Every option gives you the same <code>termage</code> command.</p>
<div class="installs">
<div class="install">
<h3 class="c10">LINUX / MACOS</h3>
<p class="dim">Downloads the right program for your machine. x86_64 and arm64.</p>
<pre class="prompt"><span class="c10">$</span> <span class="cmd">curl -fsSL {{SITE_URL}}install.sh | sh</span></pre>
<button type="button" class="copy" data-copy="curl -fsSL {{SITE_URL}}install.sh | sh">COPY</button>
</div>
<div class="install">
<h3 class="c11">WINDOWS</h3>
<p class="dim">In PowerShell. Adds termage to your PATH.</p>
<pre class="prompt"><span class="c10">PS&gt;</span> <span class="cmd">irm {{SITE_URL}}install.ps1 | iex</span></pre>
<button type="button" class="copy" data-copy="irm {{SITE_URL}}install.ps1 | iex">COPY</button>
</div>
<div class="install">
<h3 class="c14">RUST / CARGO</h3>
<p class="dim">Builds from source on any platform. Needs Rust.</p>
<pre class="prompt"><span class="c10">$</span> <span class="cmd">cargo install termage</span></pre>
<button type="button" class="copy" data-copy="cargo install termage">COPY</button>
</div>
<div class="install">
<h3 class="c13">DOWNLOAD BY HAND</h3>
<p class="dim">Grab the file for your system, rename it <code>termage</code> and make it executable. A <code>SHA256SUMS</code> file is alongside.</p>
<p><a href="{{REPO_URL}}/releases/latest">[ Latest release ]</a></p>
</div>
</div>
<p class="dim">Already installed? <code>termage --update</code> upgrades it in place. Set <code>TERMAGE_INSTALL_DIR</code> or <code>TERMAGE_VERSION</code> before the install command to choose the folder or release.</p>
</section>

<section id="try">
<h2>&gt; TRY IT HERE</h2>
<p>This terminal runs the real termage library, compiled to WebAssembly. Type
a command, press a button, or <b class="c14">drag and drop your own image, gif
or video</b> onto it. Nothing is uploaded: it all stays in your browser.</p>
<div class="chips" id="chips">{{CHIPS}}</div>
<div class="window" id="window">
<div class="titlebar"><span>TERMAGE.EXE</span><span class="dim" id="status">loading ...</span></div>
<div class="term" id="term" aria-live="polite">
<div id="output"></div>
<form id="line" autocomplete="off"><span class="c10">$&nbsp;</span><input id="input" type="text" spellcheck="false" autocapitalize="off" autocomplete="off" aria-label="terminal input"></form>
</div>
<div class="drop" id="drop">DROP FILES TO VIEW THEM</div>
</div>
<p class="dim">Files here: {{FILES}}. Try <code>ls</code>, <code>help</code> or press TAB to complete a name. Video needs a browser that can play it; the command line tool uses ffmpeg instead.</p>
</section>

<section>
<h2>&gt; WHAT IT DOES</h2>
<pre class="box">
+--------------------------------------------------------------+
| Every terminal cell shows TWO pixels, using a half block.    |
| Colours are matched to the 16 ANSI colours, and --dither     |
| mixes them to fake the ones in between.                      |
+--------------------------------------------------------------+
</pre>
<ul>
<li><b class="c10">Images.</b> PNG, JPEG, GIF, BMP and WebP, scaled to fit with their proportions kept. Transparency is drawn as black.</li>
<li><b class="c11">Animated gifs.</b> Played in place, with the right frame timing.</li>
<li><b class="c12">Video.</b> <code>termage clip.mp4</code> plays through ffmpeg.</li>
<li><b class="c13">Web addresses.</b> <code>termage https://example.com/cat.gif</code></li>
<li><b class="c14">A library too.</b> <code>termage::show</code>, <code>to_display</code> and friends, built on <a href="https://crates.io/crates/terminal_graphics">terminal_graphics</a>.</li>
</ul>
</section>

<section>
<h2>&gt; PICTURES</h2>
<div class="gallery">
<figure><img src="shots/sunset.png" alt="a sunset drawn with 16 colours" loading="lazy"><figcaption>termage sunset.jpg --dither</figcaption></figure>
<figure><img src="shots/mandelbrot.png" alt="the mandelbrot set" loading="lazy"><figcaption>termage mandelbrot.png</figcaption></figure>
<figure><img src="shots/crab.png" alt="a crab" loading="lazy"><figcaption>termage crab.png --dither</figcaption></figure>
<figure><img src="shots/plasma.gif" alt="an animated plasma" loading="lazy"><figcaption>termage plasma.gif --dither</figcaption></figure>
</div>
</section>

<section>
<h2>&gt; PALETTE</h2>
<div class="palette">{{PALETTE}}</div>
</section>

<section>
<h2>&gt; LINKS</h2>
<ul class="links">
<li><a href="{{REPO_URL}}">GitHub repository</a> - source, issues, changelog</li>
<li><a href="https://crates.io/crates/termage">crates.io/crates/termage</a></li>
<li><a href="https://docs.rs/termage">docs.rs/termage</a> - API documentation</li>
</ul>
</section>

<footer>
<p class="dim">termage v{{VERSION}} - MIT OR Apache-2.0 - <a href="{{REPO_URL}}">view the source on GitHub</a></p>
<p class="dim">NO CARRIER<span class="cursor">_</span></p>
</footer>

</main>
</div>
<script>
const SAMPLES = {{SAMPLE_LIST}};
const VERSION = "{{VERSION}}";
{{JS}}
</script>
</body>
</html>
"####;

const CSS: &str = r####"
:root { {{VARS}} }
* { box-sizing: border-box; }
html { background: #000; }
body { margin: 0; background: #000; color: var(--c7); font: 16px/1.5 "Courier New", Courier, monospace; }
.crt { position: relative; min-height: 100vh; }
.crt::after { content: ""; position: fixed; inset: 0; pointer-events: none;
  background: repeating-linear-gradient(to bottom, rgba(0,0,0,0) 0, rgba(0,0,0,0) 2px, rgba(0,0,0,.28) 3px);
  box-shadow: inset 0 0 140px rgba(0,0,0,.8); }
main { max-width: 860px; margin: 0 auto; padding: 24px 16px 64px; text-shadow: 0 0 4px currentColor; }
a { color: var(--c14); }
a:hover { background: var(--c14); color: #000; text-shadow: none; }
h1 { margin: 8px 0 0; font-size: clamp(26px, 8vw, 56px); letter-spacing: .2em; line-height: 1.2; }
h1 span { animation: rainbow 1.4s steps(1) infinite; }
@keyframes rainbow { {{KEYFRAMES}} }
h2 { color: var(--c11); font-size: 18px; margin: 40px 0 8px; }
.subtitle { color: var(--c8); margin: 4px 0 12px; }
.dim { color: var(--c8); font-size: 13px; }
.nav a { margin-right: 12px; }
.prompt { background: #000; border: 1px solid var(--c8); padding: 8px 12px; overflow-x: auto; }
.installs { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 16px; margin: 12px 0; }
.install { border: 1px solid var(--c8); padding: 8px 12px 12px; }
.install h3 { margin: 4px 0; font-size: 16px; }
.install p { margin: 4px 0 8px; }
.install .prompt { margin: 0 0 8px; white-space: pre-wrap; overflow-wrap: anywhere; }
.copy { font: inherit; background: #000; color: var(--c10); border: 1px solid var(--c10); padding: 2px 10px; cursor: pointer; }
.copy:hover, .copy:focus-visible { background: var(--c10); color: #000; text-shadow: none; }
.cursor { animation: blink 1s steps(1) infinite; }
@keyframes blink { 50% { opacity: 0; } }
code { color: var(--c10); }
ul { padding-left: 20px; list-style: none; }
li::before { content: "> "; color: var(--c8); margin-left: -20px; }
.box { color: var(--c10); overflow-x: auto; }
.chips { display: flex; flex-wrap: wrap; gap: 8px; margin: 12px 0; }
.chips button { font: inherit; font-size: 13px; background: #000; color: var(--c10); border: 1px solid var(--c10); padding: 4px 10px; cursor: pointer; text-align: left; }
.chips button small { display: block; color: var(--c8); }
.chips button:hover, .chips button:focus { background: var(--c10); color: #000; outline: none; }
.chips button:hover small, .chips button:focus small { color: #000; }
.window { position: relative; border: 4px double var(--c10); background: #000; box-shadow: 0 0 24px rgba(85,255,85,.25); }
.window.over { border-color: var(--c11); box-shadow: 0 0 32px rgba(255,255,85,.5); }
.titlebar { display: flex; justify-content: space-between; background: var(--c10); color: #000; padding: 0 8px; text-shadow: none; font-weight: bold; }
.titlebar .dim { color: #040; font-weight: normal; }
.term { height: 520px; overflow-y: auto; padding: 10px 12px; font-size: 14px; line-height: 1.35; cursor: text; }
.term canvas { display: block; max-width: 100%; height: auto; image-rendering: pixelated; margin: 4px 0 8px; border: 1px solid #222; }
.term .err { color: var(--c9); }
.term .cmd { color: var(--c15); }
.term pre { margin: 0; white-space: pre-wrap; font: inherit; }
#line { display: flex; }
#input { flex: 1; min-width: 0; background: transparent; border: 0; outline: 0; color: var(--c15); font: inherit; caret-color: var(--c10); padding: 0; text-shadow: inherit; }
.drop { display: none; position: absolute; inset: 24px 0 0 0; align-items: center; justify-content: center; background: rgba(0,0,0,.85); color: var(--c11); font-size: 22px; letter-spacing: .1em; pointer-events: none; }
.window.over .drop { display: flex; }
.gallery { display: grid; grid-template-columns: repeat(2, 1fr); gap: 12px; }
.gallery figure { margin: 0; border: 1px solid var(--c8); padding: 6px; }
.gallery img { display: block; width: 100%; height: auto; image-rendering: pixelated; }
.gallery figcaption { font-size: 12px; color: var(--c8); margin-top: 4px; }
.palette { display: grid; grid-template-columns: repeat(8, 1fr); gap: 4px; }
.palette div { padding: 14px 2px 4px; font-size: 11px; text-align: center; border: 1px solid var(--c8); text-shadow: none; }
{{CLASSES}}
@media (max-width: 600px) { .palette { grid-template-columns: repeat(4, 1fr); } .gallery { grid-template-columns: 1fr; } .term { height: 420px; } }
@media (prefers-reduced-motion: reduce) { h1 span, .cursor { animation: none; } }
"####;

const JS: &str = r####"
(function () {
  "use strict";
  var CELL_W = 8, CELL_H = 16, MAX_VIDEO_FRAMES = 150, VIDEO_EXTENSIONS = /\.(mp4|m4v|mov|mkv|webm|avi|mpg|mpeg|ogv|3gp)$/i;
  var output = document.getElementById("output");
  var term = document.getElementById("term");
  var input = document.getElementById("input");
  var statusEl = document.getElementById("status");
  var windowEl = document.getElementById("window");
  var wasm = null, palette32 = [];
  var files = {};            // name -> { bytes, url, type }
  var history = [], historyAt = 0, running = 0, timers = [];

  SAMPLES.forEach(function (name) { files[name] = { url: "samples/" + name, builtin: true }; });

  // ---- output helpers ----
  function line(text, className) {
    var div = document.createElement("div");
    if (className) div.className = className;
    div.textContent = text;
    output.appendChild(div);
    term.scrollTop = term.scrollHeight;
    return div;
  }
  function pre(text) {
    var block = document.createElement("pre");
    block.textContent = text;
    output.appendChild(block);
    term.scrollTop = term.scrollHeight;
  }
  function fail(text) { line(text, "err"); }

  var HELP = [
    "Usage: termage [OPTIONS] <INPUT>",
    "",
    "Arguments:",
    "  <INPUT>  A picture to show: an image, an animated gif or a video.",
    "           Here that means one of the files from `ls`, a file you have",
    "           dropped in, or an https address that allows it.",
    "",
    "Options:",
    "  -w, --width <WIDTH>    Width in columns [default: 80]",
    "  -H, --height <HEIGHT>  Height in rows [default: 24]",
    "  -l, --loops <N>        Play a gif or video this many times",
    "                         [default: gifs until the next command, video once]",
    "      --fps <N>          Frames per second for video [default: 10]",
    "  -d, --dither           Dither for smoother gradients with 16 colours",
    "  -h, --help             Print help",
    "  -V, --version          Print version"
  ].join("\n");

  var SHELL_HELP = "Commands: termage <file> [options], ls, clear, help.\nTry: termage crab.png    or drop a picture onto this window.";

  // ---- wasm ----
  function writeInput(bytes) {
    var ptr = wasm.alloc(bytes.length);
    new Uint8Array(wasm.memory.buffer, ptr, bytes.length).set(bytes);
  }
  function wasmError() {
    var text = new TextDecoder().decode(new Uint8Array(wasm.memory.buffer, wasm.error_ptr(), wasm.error_len()));
    return text || "could not decode the image";
  }
  // Paints the cells of the last rendered frame; each cell is a half block:
  // the background colour on top and the foreground colour below.
  function paint() {
    var cols = wasm.out_cols(), rows = wasm.out_rows();
    var cells = new Uint32Array(wasm.memory.buffer, wasm.cells_ptr(), cols * rows * 2);
    var data = new ImageData(cols * CELL_W, rows * CELL_H);
    var pixels = new Uint32Array(data.data.buffer);
    var half = CELL_H / 2, width = cols * CELL_W;
    for (var y = 0; y < rows; y++) {
      for (var x = 0; x < cols; x++) {
        var i = (y * cols + x) * 2, colour = cells[i + 1];
        var top = palette32[(colour >> 8) & 255], bottom = palette32[colour & 255];
        for (var dy = 0; dy < CELL_H; dy++) {
          var value = dy < half ? top : bottom, offset = (y * CELL_H + dy) * width + x * CELL_W;
          for (var dx = 0; dx < CELL_W; dx++) pixels[offset + dx] = value;
        }
      }
    }
    return data;
  }
  function makeCanvas(first) {
    var canvas = document.createElement("canvas");
    canvas.width = first.width; canvas.height = first.height;
    canvas.getContext("2d").putImageData(first, 0, 0);
    output.appendChild(canvas);
    term.scrollTop = term.scrollHeight;
    return canvas;
  }
  // Shows frames on one canvas, looping `loops` times (0 = until the next command).
  function animate(canvas, frames, delays, loops) {
    if (frames.length < 2) return;
    var context = canvas.getContext("2d"), index = 0, played = 0;
    var entry = { timer: null, stopped: false };
    timers.push(entry);
    function tick() {
      if (entry.stopped) return;
      index = (index + 1) % frames.length;
      if (index === 0 && ++played >= loops && loops > 0) return;
      context.putImageData(frames[index], 0, 0);
      entry.timer = setTimeout(tick, delays[index]);
    }
    entry.timer = setTimeout(tick, delays[0]);
  }
  function stopAnimations() { timers.forEach(function (t) { t.stopped = true; clearTimeout(t.timer); }); timers = []; }

  // ---- loading files ----
  function fetchBytes(entry) {
    if (entry.bytes) return Promise.resolve(entry.bytes);
    return fetch(entry.url).then(function (r) {
      if (!r.ok) throw new Error("HTTP " + r.status);
      return r.arrayBuffer();
    }).then(function (buffer) { entry.bytes = new Uint8Array(buffer); return entry.bytes; });
  }
  function isVideo(name, entry) {
    return VIDEO_EXTENSIONS.test(name) || (entry.type && entry.type.indexOf("video/") === 0);
  }
  function once(target, name) {
    return new Promise(function (resolve, reject) {
      target.addEventListener(name, resolve, { once: true });
      target.addEventListener("error", function () { reject(new Error("the browser cannot play this video")); }, { once: true });
    });
  }
  function videoFrames(entry, opts, notice) {
    var url = entry.bytes
      ? (entry.blobUrl || (entry.blobUrl = URL.createObjectURL(new Blob([entry.bytes], { type: entry.type || "video/mp4" }))))
      : entry.url;
    var video = document.createElement("video");
    video.muted = true; video.preload = "auto"; video.src = url;
    return once(video, "loadedmetadata").then(function () {
      var packed = wasm.fit_pack(video.videoWidth, video.videoHeight, opts.cols, opts.rows);
      var width = packed & 0xffff, height = packed >>> 16;
      var scratch = document.createElement("canvas");
      scratch.width = width; scratch.height = height;
      var context = scratch.getContext("2d", { willReadFrequently: true });
      var count = Math.max(1, Math.min(Math.floor((video.duration || 1) * opts.fps), MAX_VIDEO_FRAMES));
      var frames = [], index = 0;
      function step() {
        if (index >= count) return frames;
        notice.textContent = "decoding video: frame " + (index + 1) + " of " + count;
        video.currentTime = Math.min(index / opts.fps, Math.max(video.duration - 0.01, 0));
        return once(video, "seeked").then(function () {
          context.drawImage(video, 0, 0, width, height);
          var rgba = context.getImageData(0, 0, width, height).data;
          writeInput(rgba);
          if (!wasm.render_rgba(width, height, opts.cols, opts.rows, opts.dither ? 1 : 0)) throw new Error("bad frame");
          frames.push(paint());
          index++;
          return step();
        });
      }
      return step();
    });
  }

  // ---- the termage command ----
  function parseArgs(args) {
    var opts = { cols: 80, rows: 24, loops: null, fps: 10, dither: false, input: null, help: false, version: false };
    var number = function (flag, value) {
      var n = Number(value);
      if (!/^\d+$/.test(String(value)) || n < 1) throw new Error("invalid value '" + value + "' for '" + flag + "': expected a whole number above 0");
      return n;
    };
    for (var i = 0; i < args.length; i++) {
      var a = args[i], eq = a.indexOf("="), value;
      var takes = function (names) {
        if (names.indexOf(eq > 0 ? a.slice(0, eq) : a) < 0) return false;
        if (eq > 0) value = a.slice(eq + 1);
        else if (i + 1 < args.length) value = args[++i];
        else throw new Error("a value is required for '" + a + "' but none was supplied");
        return true;
      };
      if (a === "-h" || a === "--help") opts.help = true;
      else if (a === "-V" || a === "--version") opts.version = true;
      else if (a === "-d" || a === "--dither") opts.dither = true;
      else if (takes(["-w", "--width"])) opts.cols = number("--width", value);
      else if (takes(["-H", "--height"])) opts.rows = number("--height", value);
      else if (takes(["-l", "--loops"])) opts.loops = number("--loops", value);
      else if (takes(["--fps"])) opts.fps = number("--fps", value);
      else if (a.charAt(0) === "-" && a.length > 1) throw new Error("unexpected argument '" + a + "' found");
      else if (opts.input === null) opts.input = a;
      else throw new Error("unexpected argument '" + a + "' found");
    }
    return opts;
  }

  function termage(args) {
    var opts;
    try { opts = parseArgs(args); } catch (e) { fail("error: " + e.message); line("Usage: termage [OPTIONS] <INPUT>"); return Promise.resolve(); }
    if (opts.help) { pre(HELP); return Promise.resolve(); }
    if (opts.version) { line("termage " + VERSION); return Promise.resolve(); }
    if (opts.input === null) { fail("error: the following required arguments were not provided:\n  <INPUT>"); line("Usage: termage [OPTIONS] <INPUT>"); return Promise.resolve(); }

    var name = opts.input, entry = files[name];
    var failure = function (message) { fail("termage: " + name + ": " + message); };

    if (!entry && /^https?:\/\//i.test(name)) {
      entry = { url: name, remote: true };
    }
    if (!entry) { failure("No such file or directory (os error 2)"); return Promise.resolve(); }

    var notice = line("");
    var done = function () { notice.remove(); };
    var download = function () {
      return fetch(entry.url).then(function (r) {
        if (!r.ok) throw new Error("HTTP status " + r.status);
        return r.arrayBuffer();
      }).then(function (buffer) { entry.bytes = new Uint8Array(buffer); }, function (e) {
        throw new Error("could not download it: " + e.message + " (browsers only allow downloads from servers that permit it)");
      });
    };

    var work;
    if (isVideo(name, entry)) {
      work = (entry.remote ? download() : Promise.resolve())
        .then(function () { return videoFrames(entry, opts, notice); })
        .then(function (frames) {
          var delay = Math.round(1000 / opts.fps);
          return { frames: frames, delays: frames.map(function () { return delay; }), loops: opts.loops || 1 };
        });
    } else {
      work = (entry.remote ? download() : fetchBytes(entry)).then(function () {
        var bytes = entry.bytes, frames = [], delays = [];
        writeInput(bytes);
        var count = wasm.decode(bytes.length);
        if (!count) throw new Error(wasmError());
        for (var i = 0; i < count; i++) {
          wasm.render(i, opts.cols, opts.rows, opts.dither ? 1 : 0);
          frames.push(paint());
          delays.push(Math.max(wasm.frame_delay(i), 20));
        }
        return { frames: frames, delays: delays, loops: opts.loops || 0 };
      });
    }
    return work.then(function (result) {
      done();
      animate(makeCanvas(result.frames[0]), result.frames, result.delays, result.loops);
    }).catch(function (e) { done(); failure(e.message); });
  }

  // ---- the little shell ----
  function run(text) {
    text = text.trim();
    if (!text) return Promise.resolve();
    var words = text.match(/"[^"]*"|\S+/g).map(function (w) { return w.replace(/^"|"$/g, ""); });
    var command = words[0], args = words.slice(1);
    if (command === "termage") return termage(args);
    if (command === "ls") { line(Object.keys(files).sort().join("  ")); return Promise.resolve(); }
    if (command === "clear") { stopAnimations(); output.innerHTML = ""; return Promise.resolve(); }
    if (command === "help") { pre(SHELL_HELP); return Promise.resolve(); }
    fail(command + ": command not found. Try `help`.");
    return Promise.resolve();
  }

  function submit(text) {
    line("$ " + text, "cmd");
    if (text.trim()) { history.push(text); historyAt = history.length; }
    running++;
    input.disabled = true;
    return run(text).then(function () {}, function (e) { fail(String(e)); }).then(function () {
      running--;
      input.disabled = false;
      input.focus({ preventScroll: true });
      term.scrollTop = term.scrollHeight;
    });
  }

  document.getElementById("line").addEventListener("submit", function (event) {
    event.preventDefault();
    var text = input.value;
    input.value = "";
    submit(text);
  });
  input.addEventListener("keydown", function (event) {
    if (event.key === "ArrowUp" && historyAt > 0) { event.preventDefault(); input.value = history[--historyAt]; }
    else if (event.key === "ArrowDown") { event.preventDefault(); historyAt = Math.min(historyAt + 1, history.length); input.value = history[historyAt] || ""; }
    else if (event.key === "Tab") {
      event.preventDefault();
      var parts = input.value.split(/\s+/), last = parts[parts.length - 1];
      var matches = Object.keys(files).filter(function (n) { return n.indexOf(last) === 0; });
      if (parts.length > 1 && matches.length === 1) { parts[parts.length - 1] = matches[0]; input.value = parts.join(" "); }
      else if (parts.length === 1 && "termage".indexOf(last) === 0) input.value = "termage ";
    }
  });
  term.addEventListener("click", function () { if (!getSelection().toString()) input.focus({ preventScroll: true }); });

  Array.prototype.forEach.call(document.querySelectorAll("#chips button"), function (button) {
    button.addEventListener("click", function () {
      if (!wasm || running) return;
      input.value = "";
      submit(button.dataset.command);
    });
  });

  // ---- drag and drop ----
  var dragDepth = 0;
  ["dragenter", "dragover"].forEach(function (name) {
    document.addEventListener(name, function (event) {
      if (!event.dataTransfer || Array.prototype.indexOf.call(event.dataTransfer.types, "Files") < 0) return;
      event.preventDefault();
      windowEl.classList.add("over");
    });
  });
  document.addEventListener("dragleave", function (event) {
    if (event.relatedTarget === null) windowEl.classList.remove("over");
  });
  document.addEventListener("drop", function (event) {
    if (!event.dataTransfer || !event.dataTransfer.files.length) return;
    event.preventDefault();
    windowEl.classList.remove("over");
    if (!wasm) return;
    Array.prototype.slice.call(event.dataTransfer.files).reduce(function (chain, file) {
      return chain.then(function () { return file.arrayBuffer(); }).then(function (buffer) {
        var name = file.name.replace(/\s+/g, "_");
        files[name] = { bytes: new Uint8Array(buffer), type: file.type };
        return submit("termage " + name);
      });
    }, Promise.resolve());
  });

  // ---- start up ----
  fetch("termage.wasm").then(function (r) { return r.arrayBuffer(); })
    .then(function (bytes) { return WebAssembly.instantiate(bytes, {}); })
    .then(function (result) {
      wasm = result.instance.exports;
      for (var i = 0; i < 16; i++) {
        var rgb = wasm.palette(i);
        palette32.push((0xff000000 | ((rgb & 255) << 16) | (rgb & 0xff00) | ((rgb >> 16) & 255)) >>> 0);
      }
      statusEl.textContent = "ready";
      line("termage " + VERSION + " - view images, gifs and video in the terminal");
      line("Type `help`, press a button above, or drop a picture on this window.");
      line("");
      input.focus({ preventScroll: true });
    })
    .catch(function (error) { statusEl.textContent = "failed"; fail("could not start termage.wasm: " + error); });
})();
document.querySelectorAll(".copy").forEach(function (button) {
  button.addEventListener("click", function () {
    var done = function () { button.textContent = "COPIED"; setTimeout(function () { button.textContent = "COPY"; }, 1500); };
    if (navigator.clipboard) { navigator.clipboard.writeText(button.dataset.copy).then(done, function () {}); }
  });
});
"####;

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The page title with every letter in its own span, so each one can cycle
/// through the bright colours.
fn rainbow_title(text: &str) -> String {
    text.chars()
        .enumerate()
        .map(|(i, letter)| {
            format!(
                "<span style=\"animation-delay:-{}s\">{letter}</span>",
                i as f32 * 0.2
            )
        })
        .collect()
}

fn hex((r, g, b): (u8, u8, u8)) -> String {
    format!("#{r:02x}{g:02x}{b:02x}")
}

fn css_variables() -> String {
    Colour::ALL
        .iter()
        .enumerate()
        .map(|(i, colour)| format!("--c{i}:{};", hex(colour.rgb())))
        .collect()
}

fn colour_classes() -> String {
    (0..Colour::ALL.len())
        .map(|i| format!(".c{i}{{color:var(--c{i});}}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Cycles `color` through the 7 bright colours (the last 7 in `Colour::ALL`).
fn rainbow_keyframes() -> String {
    let bright = &Colour::ALL[9..];
    bright
        .iter()
        .enumerate()
        .map(|(i, colour)| {
            let percent = i as f32 * 100.0 / bright.len() as f32;
            format!("{percent:.1}% {{ color: {}; }}", hex(colour.rgb()))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn palette_swatches() -> String {
    Colour::ALL
        .iter()
        .enumerate()
        .map(|(i, colour)| {
            let (r, g, b) = colour.rgb();
            let light = (r as u32 * 299 + g as u32 * 587 + b as u32 * 114) / 1000 > 120;
            format!(
                "<div style=\"background:var(--c{i});color:{}\">{}</div>",
                if light { "#000" } else { "#fff" },
                colour.name()
            )
        })
        .collect()
}

fn chips() -> String {
    EXAMPLES
        .iter()
        .map(|(command, caption)| {
            format!(
                "<button type=\"button\" data-command=\"{}\">$ {}<small>{}</small></button>",
                escape(command),
                escape(command),
                escape(caption)
            )
        })
        .collect()
}

/// The file names in `dir` that the page can show, sorted.
fn sample_names(dir: &Path) -> std::io::Result<Vec<String>> {
    let mut names: Vec<String> = fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| {
            let lower = name.to_ascii_lowercase();
            [".png", ".jpg", ".jpeg", ".gif", ".webp", ".bmp"]
                .iter()
                .any(|ext| lower.ends_with(ext))
        })
        .collect();
    names.sort();
    Ok(names)
}

/// A JSON array of strings, for the page's script.
fn json_list(names: &[String]) -> String {
    let items: Vec<String> = names
        .iter()
        .map(|name| format!("\"{}\"", name.replace('\\', "\\\\").replace('"', "\\\"")))
        .collect();
    format!("[{}]", items.join(","))
}

fn build_page(samples: &[String]) -> String {
    let css = CSS
        .replace("{{VARS}}", &css_variables())
        .replace("{{KEYFRAMES}}", &rainbow_keyframes())
        .replace("{{CLASSES}}", &colour_classes());
    // `{{JS}}` and `{{SAMPLE_LIST}}` last: they must not be searched for placeholders
    PAGE.replace("{{CSS}}", &css)
        .replace("{{TITLE}}", &rainbow_title("TERMAGE"))
        .replace("{{CHIPS}}", &chips())
        .replace("{{PALETTE}}", &palette_swatches())
        .replace(
            "{{FILES}}",
            &samples
                .iter()
                .map(|name| format!("<code>{}</code>", escape(name)))
                .collect::<Vec<_>>()
                .join(", "),
        )
        .replace("{{VERSION}}", env!("CARGO_PKG_VERSION"))
        .replace("{{SITE_URL}}", SITE_URL)
        .replace("{{REPO_URL}}", REPO_URL)
        .replace("{{SAMPLE_LIST}}", &json_list(samples))
        .replace("{{JS}}", JS)
}

/// Copies every file in `from` into `to`.
fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        if entry.path().is_file() {
            fs::copy(entry.path(), to.join(entry.file_name()))?;
        }
    }
    Ok(())
}

/// Writes the site into `out`. Returns false if the compiled module was missing.
fn build_site(out: &Path, wasm: &Path, samples: &Path, shots: &Path) -> std::io::Result<bool> {
    fs::create_dir_all(out)?;
    let names = sample_names(samples)?;
    fs::write(out.join("index.html"), build_page(&names))?;
    fs::write(out.join(".nojekyll"), "")?;
    for (name, script) in install_scripts() {
        fs::write(out.join(name), script)?;
    }
    copy_dir(samples, &out.join("samples"))?;
    copy_dir(shots, &out.join("shots"))?;
    if wasm.exists() {
        fs::copy(wasm, out.join("termage.wasm"))?;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn main() -> std::io::Result<()> {
    let out = std::env::args()
        .nth(1)
        .map_or_else(|| PathBuf::from("target/site"), PathBuf::from);
    let found = build_site(
        &out,
        Path::new(WASM_PATH),
        Path::new(SAMPLES_DIR),
        Path::new("docs/screenshots"),
    )?;
    println!("site written to {}", out.display());
    if !found {
        eprintln!(
            "warning: {WASM_PATH} not found, the terminal will not load. Build it with:\n  \
             cargo build --release --example web --no-default-features --target wasm32-unknown-unknown"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page() -> String {
        build_page(&["a.png".to_string(), "b.gif".to_string()])
    }

    #[test]
    fn every_placeholder_is_filled_in() {
        let page = page();
        assert!(!page.contains("{{"), "left over placeholder in the page");
    }

    #[test]
    fn the_page_links_back_to_the_repo_and_to_the_docs() {
        let page = page();
        assert!(page.contains(&format!("href=\"{REPO_URL}\"")));
        assert!(page.contains("https://crates.io/crates/termage"));
        assert!(page.contains("https://docs.rs/termage"));
    }

    #[test]
    fn the_page_has_the_terminal_and_loads_the_wasm() {
        let page = page();
        assert!(page.contains("id=\"term\""));
        assert!(page.contains("id=\"input\""));
        assert!(page.contains("fetch(\"termage.wasm\")"));
    }

    #[test]
    fn every_example_command_is_a_button_and_uses_a_file_that_exists() {
        let page = page();
        let samples = sample_names(Path::new(SAMPLES_DIR)).unwrap();
        for (command, _) in EXAMPLES {
            assert!(page.contains(&format!("data-command=\"{command}\"")));
            for word in command.split_whitespace().skip(1) {
                if word.contains('.') && !word.starts_with('-') {
                    assert!(samples.contains(&word.to_string()), "{command}: no {word}");
                }
            }
        }
    }

    #[test]
    fn the_sample_list_reaches_the_script_as_json() {
        assert!(page().contains("const SAMPLES = [\"a.png\",\"b.gif\"];"));
        assert_eq!(json_list(&[]), "[]");
        assert_eq!(json_list(&["we\"ird".to_string()]), "[\"we\\\"ird\"]");
    }

    #[test]
    fn all_sixteen_colours_are_in_the_palette_with_their_names() {
        let page = page();
        for colour in Colour::ALL {
            assert!(page.contains(&format!(">{}</div>", colour.name())));
        }
        assert_eq!(css_variables().matches("--c").count(), 16);
    }

    #[test]
    fn text_is_escaped() {
        assert_eq!(escape("a<b>&\"c\""), "a&lt;b&gt;&amp;&quot;c&quot;");
    }

    #[test]
    fn only_pictures_are_listed_as_samples() {
        let dir = std::env::temp_dir().join(format!("termage_samples_list_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        for name in ["z.png", "a.GIF", "notes.txt", "m.jpeg"] {
            fs::write(dir.join(name), "").unwrap();
        }
        assert_eq!(sample_names(&dir).unwrap(), ["a.GIF", "m.jpeg", "z.png"]);
        fs::remove_dir_all(&dir).unwrap();
    }

    const ASSETS: &[&str] = &[
        "termage-x86_64-unknown-linux-gnu",
        "termage-aarch64-unknown-linux-gnu",
        "termage-x86_64-apple-darwin",
        "termage-aarch64-apple-darwin",
        "termage-x86_64-pc-windows-msvc.exe",
    ];

    #[test]
    fn the_page_shows_every_way_to_install() {
        let page = page();
        assert!(page.contains(r#"id="install""#));
        assert!(page.contains(&format!("curl -fsSL {SITE_URL}install.sh | sh")));
        assert!(page.contains(&format!("irm {SITE_URL}install.ps1 | iex")));
        assert!(page.contains("cargo install termage"));
        assert!(page.contains(&format!("{REPO_URL}/releases/latest")));
        assert!(page.contains("termage --update"));
        // every copy button has a command to copy
        assert_eq!(page.matches(r#"class="copy""#).count(), 3);
        assert_eq!(page.matches("data-copy=").count(), 3);
    }

    #[test]
    fn the_install_scripts_have_everything_filled_in() {
        for (name, script) in install_scripts() {
            assert!(!script.contains("{{"), "{name} has a placeholder left");
            assert!(script.contains(REPO_PATH), "{name}");
            assert!(script.contains("releases/latest/download/"), "{name}");
            assert!(script.contains("TERMAGE_VERSION"), "{name}");
        }
    }

    #[test]
    fn the_shell_script_picks_the_release_file_for_every_unix_machine() {
        let [(_, sh), _] = install_scripts();
        assert!(sh.starts_with("#!/bin/sh\n"));
        assert!(sh.contains("set -eu"));
        assert!(!sh.contains('\r'), "a carriage return breaks sh");
        for (arch, os) in [
            ("x86_64", "unknown-linux-gnu"),
            ("aarch64", "unknown-linux-gnu"),
            ("x86_64", "apple-darwin"),
            ("aarch64", "apple-darwin"),
        ] {
            assert!(sh.contains(&format!("arch={arch}")), "{arch}");
            assert!(sh.contains(&format!("os={os}")), "{os}");
            assert!(ASSETS.contains(&format!("termage-{arch}-{os}").as_str()));
        }
        assert!(sh.contains(r#"file="termage-$arch-$os""#));
    }

    #[test]
    fn the_powershell_script_downloads_the_windows_release_file() {
        let [_, (_, ps1)] = install_scripts();
        assert!(ps1.contains(ASSETS[4]));
        assert!(ps1.contains("LOCALAPPDATA"));
    }

    #[test]
    fn the_site_is_written_with_and_without_the_module() {
        let dir = std::env::temp_dir().join(format!("termage_site_{}", std::process::id()));
        let samples = dir.join("samples");
        let shots = dir.join("shots");
        fs::create_dir_all(&samples).unwrap();
        fs::create_dir_all(&shots).unwrap();
        fs::write(samples.join("pic.png"), "png").unwrap();
        fs::write(shots.join("shot.png"), "shot").unwrap();
        let wasm = dir.join("fake.wasm");

        let out = dir.join("out");
        assert!(!build_site(&out, &wasm, &samples, &shots).unwrap());
        assert!(out.join("index.html").exists());
        assert!(out.join("install.sh").exists());
        assert!(out.join("install.ps1").exists());
        assert!(out.join("samples/pic.png").exists());
        assert!(out.join("shots/shot.png").exists());
        assert!(!out.join("termage.wasm").exists());

        fs::write(&wasm, b"\0asm").unwrap();
        assert!(build_site(&out, &wasm, &samples, &shots).unwrap());
        assert_eq!(fs::read(out.join("termage.wasm")).unwrap(), b"\0asm");

        fs::remove_dir_all(&dir).unwrap();
    }
}
