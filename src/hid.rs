extern crate hidapi;
use crate::utils::crc8;
use crate::utils::random_byte;
use hidapi::HidDevice;
use std::thread;
use std::time::Duration;

pub fn write_to_device(device: &HidDevice, command: u8) {
  let mut req = [0u8; 64];
  req[0..3].copy_from_slice(&[0x3f, random_byte(), command]);
  req[63] = crc8(&req[1..63]);

  if device.write(&req).is_err() {
    eprintln!("Failed to write to device");
  }
}

pub fn read_from_device(device: &HidDevice) -> [u8; 64] {
  let mut res = [0u8; 64];
  if device.read(&mut res).is_err() {
    eprintln!("Failed to read from device");
  }
  res
}

pub fn set_color(device: &HidDevice, color: u32) {
  let mut req = [0u8; 64];
  req[0..2].copy_from_slice(&[0x3f, random_byte() | 0b100]);

  for i in 0..16 {
    req[(i * 3) + 3] = ((color >> 8) & 0x000000FF) as u8;
    req[(i * 3) + 4] = ((color >> 16) & 0x000000FF) as u8;
    req[(i * 3) + 5] = (color & 0x000000FF) as u8;
  }
  req[63] = crc8(&req[1..63]);

  if device.write(&req).is_err() {
    eprintln!("Failed to write to device");
  }

  thread::sleep(Duration::from_millis(5));
}

pub fn set_colors(device: &HidDevice, colors: Vec<String>) {
  let mut req = [0u8; 64];
  req[0..2].copy_from_slice(&[0x3f, random_byte() | 0b100]);
  let colors = colors;
  for i in 0..16 {
    let color = u32::from_str_radix(&colors[i], 16).expect("Invalid hex color");
    req[(i * 3) + 3] = ((color >> 8) & 0x000000FF) as u8;
    req[(i * 3) + 4] = ((color >> 16) & 0x000000FF) as u8;
    req[(i * 3) + 5] = (color & 0x000000FF) as u8;
  }
  req[63] = crc8(&req[1..63]);

  if device.write(&req).is_err() {
    eprintln!("Failed to write to device");
  }

  thread::sleep(Duration::from_millis(5));
}

pub fn get_device(vendor_id: u16, product_id: u16) -> Option<HidDevice> {
  let api = hidapi::HidApi::new().expect("Failed to create HID API");
  for dev in api.device_list() {
    if dev.vendor_id() == vendor_id && dev.product_id() == product_id {
      if let Ok(device) = dev.open_device(&api) {
        return Some(device);
      }
    }
  }
  eprintln!("Device not found");
  return None;
}
