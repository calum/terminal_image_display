use std::io::{self, ErrorKind};
use std::process::ExitCode;

use clap::Parser;
use termage::{pick_size, show, Error, Options, DEFAULT_FPS};

/// Display any image, animated gif or video in the terminal with Termage!
#[derive(Parser)]
#[command(version, author = "https://github.com/calum/terminal_image_display")]
struct Cli {
    /// The picture to show: a file or an http(s) address. PNG, JPEG, GIF, BMP
    /// and WebP images work, and animated gifs play. Videos (mp4, mkv, webm,
    /// ...) play silently if ffmpeg is installed.
    input: String,

    /// Width in columns [default: the width of the terminal]
    #[arg(short, long)]
    width: Option<u32>,

    /// Height in rows [default: the height of the terminal]
    #[arg(short = 'H', long)]
    height: Option<u32>,

    /// Play a gif or video this many times [default: gifs until stopped, videos once]
    #[arg(short, long, value_name = "N")]
    loops: Option<u32>,

    /// Frames per second for video
    #[arg(long, default_value_t = DEFAULT_FPS, value_name = "N")]
    fps: u32,

    /// Dither to make gradients and photos look smoother with only 16 colours
    #[arg(short, long)]
    dither: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let terminal = terminal_size::terminal_size().map(|(w, h)| (w.0, h.0));
    let (cols, rows) = pick_size(cli.width, cli.height, terminal);
    let options = Options::new(cols, rows)
        .with_loops(cli.loops)
        .with_fps(cli.fps)
        .with_dither(cli.dither);

    match show(&mut io::stdout(), &cli.input, &options) {
        Ok(()) => ExitCode::SUCCESS,
        // `termage pic.png | head` closing the pipe is not worth an error
        Err(Error::Io(error)) if error.kind() == ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("termage: {}: {error}", cli.input);
            ExitCode::FAILURE
        }
    }
}
