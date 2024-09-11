use crate::hid::{read_from_device, write_to_device};
use crate::pump::PumpMode;
use crate::utils::{byte_to_fraction, crc8, u16le_from};
use hidapi::HidDevice;
use serde_json::{json, to_string_pretty, Map};

#[derive(Debug, Clone, PartialEq)]
pub enum MeasurementType {
  String,
  Float,
  Int,
}
#[derive(Debug, Clone)]
pub struct Measurement {
  pub name: String,
  pub value: String,
  pub units: Option<String>,
  pub measurement_type: MeasurementType,
}

fn print_measurements_as_strings(measurements: &[Measurement]) {
  for measurement in measurements {
    let Measurement {
      name, value, units, ..
    } = measurement;
    let units = units.clone().unwrap_or("".to_string());
    println!("{name}: {value} {units}");
  }
}

fn print_measurements_as_json(measurements: &[Measurement]) {
  let mut json_obj = Map::new();

  for measurement in measurements {
    let mut value_obj = Map::new();
    if measurement.measurement_type == MeasurementType::Float {
      value_obj.insert(
        "value".to_string(),
        json!(measurement.value.parse::<f32>().unwrap()),
      );
    } else if measurement.measurement_type == MeasurementType::Int {
      value_obj.insert(
        "value".to_string(),
        json!(measurement.value.parse::<i32>().unwrap()),
      );
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
  println!("{json_str}");
}

pub fn get_measurements(device: &HidDevice) -> Vec<Measurement> {
  write_to_device(&device, 0xff, None, None);
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

  measurements.push(Measurement {
    name: "Fan 1 speed".to_string(),
    value: format!("{:.0}", u16le_from(&res, 15)),
    units: Some("RPM".to_string()),
    measurement_type: MeasurementType::Float,
  });

  measurements.push(Measurement {
    name: "Fan 1 duty".to_string(),
    value: format!("{:.0}", byte_to_fraction(res[14])),
    units: Some("%".to_string()),
    measurement_type: MeasurementType::Float,
  });

  measurements.push(Measurement {
    name: "Fan 2 speed".to_string(),
    value: format!("{:.0}", u16le_from(&res, 22)),
    units: Some("RPM".to_string()),
    measurement_type: MeasurementType::Float,
  });

  measurements.push(Measurement {
    name: "Fan 2 duty".to_string(),
    value: format!("{:.0}", byte_to_fraction(res[21])),
    units: Some("%".to_string()),
    measurement_type: MeasurementType::Float,
  });

  measurements
}

pub fn print_measurements(device: &HidDevice, print_json: bool) {
  let measurements = get_measurements(&device);
  if print_json {
    print_measurements_as_json(&measurements);
  } else {
    print_measurements_as_strings(&measurements);
  }
}
