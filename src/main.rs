extern crate hidapi;
use clap::Parser;
use hidapi::HidApi;
use rand::Rng;
use serde_json::json;

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
}

fn main() {
    let args = Args::parse();

    let api = HidApi::new().expect("Failed to create HID API");

    for dev in api.device_list() {
        if dev.vendor_id() == 0x1b1c && dev.product_id() == 0x0c21 {
            if let Ok(device) = dev.open_device(&api) {
                let mut req = [0u8; 64];
                req[0..3].copy_from_slice(&[0x3f, random_byte(), 0xff]);
                req[63] = crc8(&req[1..63]);

                if device.write(&req).is_err() {
                    eprintln!("Failed to write to device");
                    continue;
                }

                let mut res = [0u8; 64];
                if device.read(&mut res).is_ok() && res[63] == crc8(&res[1..63]) {
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
