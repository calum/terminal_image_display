use std::io::{self, ErrorKind};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use termage::{pick_size, show, Error};

/// Display any image or animated gif in the terminal with Termage!
#[derive(Parser)]
#[command(version, author = "https://github.com/calum/terminal_image_display")]
struct Cli {
    /// The image to show: PNG, JPEG, GIF, BMP or WebP. Animated gifs play.
    file: PathBuf,

    /// Width in columns [default: the width of the terminal]
    #[arg(short, long)]
    width: Option<u32>,

    /// Height in rows [default: the height of the terminal]
    #[arg(short = 'H', long)]
    height: Option<u32>,

    /// Play an animated gif this many times [default: until stopped]
    #[arg(short, long, value_name = "N")]
    loops: Option<u32>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let terminal = terminal_size::terminal_size().map(|(w, h)| (w.0, h.0));
    let (cols, rows) = pick_size(cli.width, cli.height, terminal);

    match show(&mut io::stdout(), &cli.file, cols, rows, cli.loops) {
        Ok(()) => ExitCode::SUCCESS,
        // `termage pic.png | head` closing the pipe is not worth an error
        Err(Error::Io(error)) if error.kind() == ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("termage: {}: {error}", cli.file.display());
            ExitCode::FAILURE
        }
    }
}
