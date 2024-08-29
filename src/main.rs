extern crate hidapi;
mod colors;
mod hid;
mod info;
mod pump;
mod utils;
use std::thread::sleep;

use clap::Parser;
use colors::{gradient, parse_color, rainbow, set_color, set_colors, set_brightness};
use hid::get_device;
use info::print_measurements;
use pump::PumpMode;

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
  /// Set the pump mode
  #[arg(short, long, value_enum)]
  pump: Option<PumpMode>,
}

fn main() {
  let args = Args::parse();
  let mut print_info = args.info;
  if let Some(device) = get_device(0x1b1c, 0x0c21) {
    if let Some(ref color_str) = args.color {
      match parse_color(color_str) {
        Ok(color) => {
          println!("Setting single color to #{:06X}", color);
          set_color(&device, color)
        }
        Err(e) => eprintln!("Error: {}", e),
      }
    }
    if let (Some(ref gradient1), Some(ref gradient2)) =
      (args.gradient1.as_ref(), args.gradient2.as_ref())
    {
      match (parse_color(gradient1), parse_color(gradient2)) {
        (Ok(start_color), Ok(end_color)) => {
          println!(
            "Setting hex color gradient from #{:06X} to #{:06X}",
            start_color, end_color
          );
          let colors = gradient(start_color, end_color);
          set_colors(&device, colors);
        }
        (Err(err), _) | (_, Err(err)) => {
          eprintln!("Error: {}", err);
        }
      }
    }
    if args.rainbow {
      println!("Setting rainbow colors");
      let colors = rainbow();
      set_colors(&device, colors);
    }
    if args.pump.is_some() {
      pump::set_pump_mode(&device, args.pump.unwrap().value());
    }
    if args.color.is_none()
      && args.gradient1.is_none()
      && !args.rainbow
      && !args.info
      && args.pump.is_none()
    {
      print_info = true;
    }
    if print_info {
      set_brightness(&device, 1);
      print_measurements(&device, args.json);
    }
  }
}
