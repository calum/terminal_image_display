extern crate image;
extern crate termage;
extern crate terminal_graphics;
extern crate terminal_size;
extern crate clap;

use image::Pixel;

use terminal_graphics::Display;
use terminal_graphics::Colour;

use terminal_size::{Width, Height, terminal_size};

use termage::{display_image, display_gif};

use clap::{Arg, App};

fn main() {
    let matches = App::new("Termage")
                    .version("1.0.1")
                    .about("Display any image in the terminal with Termage!")
                    .author("https://github.com/calum/terminal_image_display")
                    .arg(Arg::with_name("image")
                            .short("i")
                            .long("image")
                            .value_name("FILE")
                            .help("Input image filepath")
                            .conflicts_with("gif")
                            .takes_value(true))
                    .arg(Arg::with_name("gif")
                            .short("g")
                            .long("gif")
                            .value_name("FILE")
                            .help("Input animated gif filepath")
                            .takes_value(true))
                    .get_matches();

    let image_filepath = matches.value_of("image");
    let gif_filepath = matches.value_of("gif");

    let size = terminal_size();

    if let Some((Width(w), Height(h))) = size {
        if let Some(filepath) = image_filepath {
            display_image(filepath, w as u32, h as u32);
        } else if let Some(filepath) = gif_filepath {
            loop {
                display_gif(filepath, w as u32, h as u32);

            }
        }
    } else {
        println!("Error: Unable to get terminal dimensions.");
    }

}
