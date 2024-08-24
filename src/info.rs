use crate::hid::{read_from_device, write_to_device};
use crate::utils::crc8;
use hidapi::HidDevice;
use serde_json::{json, to_string_pretty, Map};
use crate::pump::PumpMode;

#[derive(PartialEq)]
enum MeasurementType {
  String,
  Float,
  Int,
}

struct Measurement {
  name: String,
  value: String,
  units: Option<String>,
  measurement_type: MeasurementType,
}

fn print_measurements_as_strings(measurements: &[Measurement]) {
  for measurement in measurements {
    let units = measurement.units.as_deref().unwrap_or("");
    println!(
      "{}: {} {}",
      measurement.name,
      measurement.value,
      units
    );
  }
}

fn print_measurements_as_json(measurements: &[Measurement]) {
  let mut json_obj = Map::new();

  for measurement in measurements {
    let mut value_obj = Map::new();
    if measurement.measurement_type == MeasurementType::Float {
      value_obj.insert("value".to_string(), json!(measurement.value.parse::<f32>().unwrap()));
    } else if measurement.measurement_type == MeasurementType::Int {
      value_obj.insert("value".to_string(), json!(measurement.value.parse::<i32>().unwrap()));
    } else if measurement.measurement_type == MeasurementType::String {
      value_obj.insert("value".to_string(), json!(measurement.value));
    }
    if let Some(units) = &measurement.units {
      value_obj.insert("units".to_string(), json!(units));
    }
    let measurement_name = measurement.name.clone().to_lowercase().replace(" ", "_");
    json_obj.insert(measurement_name, json!(value_obj));
  }

  let json_str = to_string_pretty(&json_obj).unwrap();
  println!("{}", json_str);
}

fn get_measurements(device: &HidDevice) -> Vec<Measurement> {
  let res = read_from_device(&device);
  if res[63] != crc8(&res[1..63]) {
    eprintln!("CRC8 check failed or read error");
    return Vec::new();
  }

  let mut measurements = vec![];

  if let Ok(Some(manufacturer)) = device.get_manufacturer_string() {
    measurements.push(Measurement {
      name: "Manufacturer".to_string(),
      value: manufacturer,
      units: None,
      measurement_type: MeasurementType::String,
    });
  }

  if let Ok(Some(product)) = device.get_product_string() {
    measurements.push(Measurement {
      name: "Product".to_string(),
      value: product,
      units: None,
      measurement_type: MeasurementType::String,
    });
  }

  measurements.push(Measurement {
    name: "Firmware".to_string(),
    value: format!("{}.{}.{}", res[2] >> 4, res[2] & 0xf, res[3]),
    units: None,
    measurement_type: MeasurementType::String,
  });

  measurements.push(Measurement {
    name: "Liquid temperature".to_string(),
    value: format!("{:.2}", res[8] as f32 + res[7] as f32 / 255.0),
    units: Some("°C".to_string()),
    measurement_type: MeasurementType::Float,
  });

  measurements.push(Measurement {
    name: "Pump speed".to_string(),
    value: format!("{:.2}", res[28] as f32 / 255.0 * 100.0),
    units: Some("%".to_string()),
    measurement_type: MeasurementType::Float,
  });

  measurements.push(Measurement {
    name: "Pump mode".to_string(),
    value: format!("{:?}", PumpMode::from_u8(res[24] as u8)),
    units: None,
    measurement_type: MeasurementType::String,
  });

  measurements
}

pub fn print_measurements(device: &HidDevice, print_json: bool) {
  write_to_device(&device, 0xff, None, None);
  let measurements = get_measurements(&device);
  if print_json {
    print_measurements_as_json(&measurements);
  } else {
    print_measurements_as_strings(&measurements);
  }
}
