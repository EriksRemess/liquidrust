use crate::utils::{crc8, random_byte};
use hidapi::{HidApi, HidDevice};

pub fn write_to_device(
  device: &HidDevice,
  command: u8,
  feature: Option<u8>,
  data: Option<&[u8; 60]>,
) {
  let mut req = [0u8; 64];
  let mut start_at = 2;
  if let Some(feature) = feature {
    start_at = 3;
    req[0..3].copy_from_slice(&[0x3f, random_byte() | feature, command]);
  } else {
    req[0..2].copy_from_slice(&[0x3f, random_byte() | command]);
  }
  if let Some(data) = data {
    req[start_at..start_at + 60].copy_from_slice(data);
  }
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

pub fn get_device(vendor_id: u16, product_id: u16) -> Option<HidDevice> {
  let api = HidApi::new().expect("Failed to create HID API");
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
