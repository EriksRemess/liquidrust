extern crate hidapi;
mod colors;
mod hid;
mod info;
mod utils;
use clap::Parser;
use colors::gradient;
use colors::rainbow;
use hid::get_device;
use hid::set_color;
use hid::set_colors;
use info::print_measurements;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
  /// Output in JSON format
  #[arg(short, long)]
  json: bool,
  /// RGB color
  #[arg(short, long)]
  color: Option<String>,
  /// Gradient color 1
  #[arg(short = 'a', long)]
  gradient1: Option<String>,
  /// Gradient color 2
  #[arg(short = 'b', long)]
  gradient2: Option<String>,
  /// Rainbow
  #[arg(short, long)]
  rainbow: bool,
  /// Device info & measurements
  #[arg(short, long)]
  info: bool,
}

#[warn(unused_variables)]

fn main() {
  let args = Args::parse();
  let mut print_info = args.info;
  if let Some(device) = get_device(0x1b1c, 0x0c21) {
    if let Some(ref color_str) = args.color {
      let color_value = u32::from_str_radix(&color_str, 16).expect("Invalid hex color");
      set_color(&device, color_value);
    }
    if let (Some(ref gradient1_str), Some(ref gradient2_str)) =
      (args.gradient1.as_ref(), args.gradient2.as_ref())
    {
      let colors = gradient(gradient1_str, gradient2_str);
      set_colors(&device, colors);
    }
    if args.rainbow {
      let colors = rainbow();
      set_colors(&device, colors);
    }
    if !args.color.is_some() && !args.gradient1.is_some() && !args.rainbow && !args.info {
      print_info = true;
    }
    if print_info {
      print_measurements(&device, args.json);
    }
    // no command specified

  }
}
