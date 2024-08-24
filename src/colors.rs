const COLOR_COUNT: usize = 16;

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
  for i in 0..COLOR_COUNT {
    let hue = (i as f64 / COLOR_COUNT as f64) * 360.0;
    let (r, g, b) = hsl_to_rgb(hue, 1.0, 0.5);
    colors.push(rgb_to_hex(r, g, b));
  }
  colors
}

pub fn gradient(start_color: &str, end_color: &str) -> Vec<String> {
  let start_color = (
    u8::from_str_radix(&start_color[0..2], 16).unwrap(),
    u8::from_str_radix(&start_color[2..4], 16).unwrap(),
    u8::from_str_radix(&start_color[4..6], 16).unwrap(),
  );
  let end_color = (
    u8::from_str_radix(&end_color[0..2], 16).unwrap(),
    u8::from_str_radix(&end_color[2..4], 16).unwrap(),
    u8::from_str_radix(&end_color[4..6], 16).unwrap(),
  );
  let mut colors = Vec::new();
  for i in 0..COLOR_COUNT {
    let factor = i as f64 / (COLOR_COUNT as f64 - 1.0);
    let (r, g, b) = interpolate_color(start_color, end_color, factor);
    colors.push(rgb_to_hex(r, g, b));
  }
  colors
}
