extern crate hidapi;
use clap::Parser;
use hidapi::HidApi;
use hidapi::HidDevice;
use rand::Rng;
use serde_json::json;
use std::thread;
use std::time::Duration;

fn crc8(data: &[u8]) -> u8 {
    data.iter().fold(0x00, |mut crc, &byte| {
        crc ^= byte;
        for _ in 0..8 {
            crc = if crc & 0x80 != 0 {
                (crc << 1) ^ 0x07
            } else {
                crc << 1
            };
        }
        crc
    })
}

fn random_byte() -> u8 {
    rand::thread_rng().gen_range(1..=31) << 3
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(first) => {
            first.to_uppercase().collect::<String>() + c.as_str().to_lowercase().as_str()
        }
    }
}

struct Measurement {
    name: String,
    value: String,
    units: Option<String>,
}

fn print_measurements(measurements: &[Measurement]) {
    for measurement in measurements {
        let units = measurement.units.as_deref().unwrap_or("");
        println!(
            "{}: {} {}",
            capitalize(&measurement.name),
            measurement.value,
            units
        );
    }
}

fn print_measurements_as_json(measurements: &[Measurement]) {
    let mut json_obj = serde_json::Map::new();

    for measurement in measurements {
        let mut value_obj = serde_json::Map::new();
        value_obj.insert("value".to_string(), json!(measurement.value));
        if let Some(units) = &measurement.units {
            value_obj.insert("units".to_string(), json!(units));
        }
        json_obj.insert(measurement.name.clone(), json!(value_obj));
    }

    let json_str = serde_json::to_string_pretty(&json_obj).unwrap();
    println!("{}", json_str);
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Output in JSON format
    #[arg(short, long)]
    json: bool,
    /// RGB color
    #[arg(short, long)]
    color: Option<String>,
}

fn write_to_device(device: &HidDevice, command: u8) {
    let mut req = [0u8; 64];
    req[0..3].copy_from_slice(&[0x3f, random_byte(), command]);
    req[63] = crc8(&req[1..63]);

    if device.write(&req).is_err() {
        eprintln!("Failed to write to device");
    }
}

fn read_from_device(device: &HidDevice) -> [u8; 64] {
    let mut res = [0u8; 64];
    if device.read(&mut res).is_err() {
        eprintln!("Failed to read from device");
    }
    res
}

fn set_colors(device: &HidDevice, color: u32) {
    let mut req = [0u8; 64];
    req[0..2].copy_from_slice(&[0x3f, random_byte() | 0b100]);
    let mut color = color;
    for i in 0..16 {
        // color = color.rotate_left(1);
        // color = random_rgb();
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

fn random_rgb() -> u32 {
    rand::thread_rng().gen_range(0..=0xFFFFFF)
}

fn main() {
    let args = Args::parse();

    let api = HidApi::new().expect("Failed to create HID API");

    for dev in api.device_list() {
        if dev.vendor_id() == 0x1b1c && dev.product_id() == 0x0c21 {
            if let Ok(device) = dev.open_device(&api) {
                if let Some(ref color_str) = args.color {
                    let color_value =
                        u32::from_str_radix(&color_str, 16).expect("Invalid hex color");
                    set_colors(&device, color_value);
                }
                write_to_device(&device, 0xff);
                let res = read_from_device(&device);
                if res[63] == crc8(&res[1..63]) {
                    let measurements = vec![
                        Measurement {
                            name: "liquid".to_string(),
                            value: format!("{:.2}", res[8] as f32 + res[7] as f32 / 255.0),
                            units: Some("°C".to_string()),
                        },
                        Measurement {
                            name: "pump".to_string(),
                            value: format!("{:.2}", res[28] as f32 / 255.0 * 100.0),
                            units: Some("%".to_string()),
                        },
                        Measurement {
                            name: "device".to_string(),
                            value: format!(
                                "{} {}",
                                dev.manufacturer_string().unwrap_or("Unknown"),
                                dev.product_string().unwrap_or("Unknown")
                            ),
                            units: None,
                        },
                        Measurement {
                            name: "firmware".to_string(),
                            value: format!("{}.{}.{}", res[2] >> 4, res[2] & 0xf, res[3]),
                            units: None,
                        },
                    ];

                    if args.json {
                        print_measurements_as_json(&measurements);
                    } else {
                        print_measurements(&measurements);
                    }
                } else {
                    println!("CRC8 check failed or read error");
                }
            } else {
                eprintln!("Failed to open device");
            }
        }
    }
}
