extern crate hidapi;

use hidapi::HidApi;
use rand::Rng;

fn crc8(message: &[u8], polynomial: u8, init: u8) -> u8 {
  let mut remainder = init;

  for &byte in message.iter() {
      remainder ^= byte;
      for _ in 0..8 {
          if remainder & 0x80 != 0 {
              remainder = (remainder << 1) ^ polynomial;
          } else {
              remainder = remainder << 1;
          }
      }
  }

  remainder
}

fn random_byte() -> u8 {
  let mut rng = rand::thread_rng();
  let random_number = rng.gen_range(1..=31);
  random_number << 3
}

fn main() {
    let api = HidApi::new().expect("Failed to create HID API");

    for device in api.device_list() {
        if device.vendor_id() == 0x1b1c && device.product_id() == 0x0c21 {
            match device.open_device(&api) {
                Ok(liquid) => {
                    let mut request: [u8; 64] = [
                        0x3f, random_byte(), 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    ];
                    request[request.len() - 1] = crc8(&request[1..63], 0x07, 0x00);

                    if let Err(e) = liquid.write(&request) {
                        eprintln!("Failed to write to device: {}", e);
                        continue;
                    }

                    let mut response = [0u8; 64];
                    match liquid.read(&mut response) {
                        Ok(_) => {
                            if response[63] == crc8(&response[1..63], 0x07, 0x00) {
                                println!("{:.2}°C", response[8] as f32 + response[7] as f32 / 255.0);
                            } else {
                                println!("CRC8 check failed");
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to read from device: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to open device: {}", e);
                }
            }
        }
    }
}
