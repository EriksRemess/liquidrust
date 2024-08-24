use crate::hid::write_to_device;
use hidapi::HidDevice;
use regex::Regex;
use std::thread::sleep;
use std::time::Duration;
pub const LED_COUNT: usize = 16;

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (u8, u8, u8) {
  let a = s * f64::min(l, 1.0 - l);
  let f = |n: f64| {
    let k = (n + h / 30.0) % 12.0;
    l - a * f64::max(-1.0, f64::min(f64::min(k - 3.0, 9.0 - k), 1.0))
  };
  let r = (f(0.0) * 255.0).round() as u8;
  let g = (f(8.0) * 255.0).round() as u8;
  let b = (f(4.0) * 255.0).round() as u8;
  (r, g, b)
}

fn rgb_to_hex(r: u8, g: u8, b: u8) -> String {
  format!("{:02x}{:02x}{:02x}", r, g, b)
}

pub fn u32_to_rgb(color: u32) -> (u8, u8, u8) {
  let r = ((color >> 16) & 0xff) as u8;
  let g = ((color >> 8) & 0xff) as u8;
  let b = (color & 0xff) as u8;
  (r, g, b)
}

fn interpolate(start: f64, end: f64, factor: f64) -> f64 {
  start + (end - start) * factor
}

fn interpolate_color(start: (u8, u8, u8), end: (u8, u8, u8), factor: f64) -> (u8, u8, u8) {
  let r = interpolate(start.0 as f64, end.0 as f64, factor).round() as u8;
  let g = interpolate(start.1 as f64, end.1 as f64, factor).round() as u8;
  let b = interpolate(start.2 as f64, end.2 as f64, factor).round() as u8;
  (r, g, b)
}

pub fn rainbow() -> Vec<String> {
  let mut colors = Vec::new();
  for i in 0..LED_COUNT {
    let hue = (i as f64 / LED_COUNT as f64) * 360.0;
    let (r, g, b) = hsl_to_rgb(hue, 1.0, 0.5);
    colors.push(rgb_to_hex(r, g, b));
  }
  colors
}

pub fn gradient(start_color: u32, end_color: u32) -> Vec<String> {
  let start_color = u32_to_rgb(start_color);
  let end_color = u32_to_rgb(end_color);
  let mut colors = Vec::new();
  for i in 0..(LED_COUNT / 2) {
    let factor = i as f64 / ((LED_COUNT / 2) as f64 - 1.0);
    let (r, g, b) = interpolate_color(start_color, end_color, factor);
    colors.push(rgb_to_hex(r, g, b));
  }
  let inverse_colors: Vec<String> = colors.iter().rev().map(|c| c.to_string()).collect();
  colors.extend(inverse_colors);
  colors
}

pub fn parse_color(color_str: &str) -> Result<u32, String> {
  let valid_color = Regex::new(r"^#?([A-Fa-f0-9]{6}|[A-Fa-f0-9]{3})$").unwrap();
  if !valid_color.is_match(color_str) {
    return Err("Invalid hex color".to_string());
  }
  let mut color_str = color_str.trim_start_matches('#').to_string();
  if color_str.len() == 3 {
    color_str = color_str
      .chars()
      .flat_map(|c| std::iter::repeat(c).take(2))
      .collect();
  }
  u32::from_str_radix(&color_str, 16).map_err(|_| "Failed to parse hex color".to_string())
}

pub fn set_color(device: &HidDevice, color: u32) {
  let mut data = [0u8; 60];
  let (r, g, b) = u32_to_rgb(color);
  for i in 0..LED_COUNT {
    data[(i * 3) + 1] = g;
    data[(i * 3) + 2] = r;
    data[(i * 3) + 3] = b;
  }
  write_to_device(device, 0b100, None, Some(&data));
  sleep(Duration::from_millis(5));
}

pub fn set_colors(device: &HidDevice, colors: Vec<String>) {
  let mut data = [0u8; 60];
  let colors = colors;
  for i in 0..LED_COUNT {
    let color = parse_color(&colors[i]).unwrap();
    let (r, g, b) = u32_to_rgb(color);
    data[(i * 3) + 1] = g;
    data[(i * 3) + 2] = r;
    data[(i * 3) + 3] = b;
  }
  write_to_device(device, 0b100, None, Some(&data));
  sleep(Duration::from_millis(5));
}
