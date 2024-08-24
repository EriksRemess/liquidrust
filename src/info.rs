use crate::hid::read_from_device;
use crate::hid::write_to_device;
use crate::utils::capitalize;
use crate::utils::crc8;
use serde_json::json;

pub struct Measurement {
  pub name: String,
  pub value: String,
  pub units: Option<String>,
}

pub fn print_measurements_as_strings(measurements: &[Measurement]) {
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

pub fn print_measurements_as_json(measurements: &[Measurement]) {
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

pub fn get_measurements(device: &hidapi::HidDevice) -> Vec<Measurement> {
  let res = read_from_device(&device);
  if res[63] != crc8(&res[1..63]) {
    eprintln!("CRC8 check failed or read error");
    return Vec::new();
  }

  let mut measurements = vec![];

  if let Ok(Some(manufacturer)) = device.get_manufacturer_string() {
    measurements.push(Measurement {
      name: "manufacturer".to_string(),
      value: manufacturer,
      units: None,
    });
  }

  if let Ok(Some(product)) = device.get_product_string() {
    measurements.push(Measurement {
      name: "product".to_string(),
      value: product,
      units: None,
    });
  }

  measurements.push(Measurement {
    name: "firmware".to_string(),
    value: format!("{}.{}.{}", res[2] >> 4, res[2] & 0xf, res[3]),
    units: None,
  });

  measurements.push(Measurement {
    name: "liquid".to_string(),
    value: format!("{:.2}", res[8] as f32 + res[7] as f32 / 255.0),
    units: Some("°C".to_string()),
  });

  measurements.push(Measurement {
    name: "pump".to_string(),
    value: format!("{:.2}", res[28] as f32 / 255.0 * 100.0),
    units: Some("%".to_string()),
  });

  measurements
}

pub fn print_measurements(device: &hidapi::HidDevice, print_json: bool) {
  write_to_device(&device, 0xff);
  let measurements = get_measurements(&device);
  if print_json {
    print_measurements_as_json(&measurements);
  } else {
    print_measurements_as_strings(&measurements);
  }
}
