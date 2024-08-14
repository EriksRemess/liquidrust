extern crate hidapi;
use hidapi::HidApi;
use rand::Rng;

fn crc8(data: &[u8]) -> u8 {
    data.iter().fold(0x00, |mut remainder, &byte| {
        remainder ^= byte;
        for _ in 0..8 {
            remainder = if remainder & 0x80 != 0 {
                (remainder << 1) ^ 0x07
            } else {
                remainder << 1
            };
        }
        remainder
    })
}

fn random_byte() -> u8 {
    rand::thread_rng().gen_range(1..=31) << 3
}

fn main() {
    let api = HidApi::new().expect("Failed to create HID API");

    for device in api.device_list() {
        if device.vendor_id() == 0x1b1c && device.product_id() == 0x0c21 {
            if let Ok(liquid) = device.open_device(&api) {
                let mut request = [0u8; 64];
                request[0..3].copy_from_slice(&[0x3f, random_byte(), 0xff]);
                request[63] = crc8(&request[1..63]);

                if liquid.write(&request).is_err() {
                    eprintln!("Failed to write to device");
                    continue;
                }

                let mut response = [0u8; 64];
                if liquid.read(&mut response).is_ok() {
                    if response[63] == crc8(&response[1..63]) {
                        let temperature = response[8] as f32 + response[7] as f32 / 255.0;
                        println!("{:.2}°C", temperature);
                    } else {
                        println!("CRC8 check failed");
                    }
                } else {
                    eprintln!("Failed to read from device");
                }
            } else {
                eprintln!("Failed to open device");
            }
        }
    }
}
