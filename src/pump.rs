use crate::hid::write_to_device;
use clap::ValueEnum;
use hidapi::HidDevice;
use crate::info::get_measurements;

pub const PUMP_MODE_OFFSET: u8 = 0x17 - 3;

#[derive(Debug, ValueEnum, Clone, Copy)]
pub enum PumpMode {
  Quiet = 0x0,
  Balanced = 0x1,
  Extreme = 0x2,
}

impl PumpMode {
  pub fn from_u8(mode: u8) -> PumpMode {
    match mode {
      0x0 => PumpMode::Quiet,
      0x1 => PumpMode::Balanced,
      0x2 => PumpMode::Extreme,
      _ => PumpMode::Balanced,
    }
  }

  pub fn from_str(mode: String) -> PumpMode {
      match mode.to_lowercase().as_str() {
        "quiet" => PumpMode::Quiet,
        "balanced" => PumpMode::Balanced,
        "extreme" => PumpMode::Extreme,
        _ => PumpMode::Balanced,
      }
    }

  pub fn value(&self) -> u8 {
    *self as u8
  }
}

pub fn set_pump_mode(device: &HidDevice, mode: u8) {
  let mut data = [0u8; 60];
  data[0..8].copy_from_slice(&[0x0, 0xff, 0x5, 0xff, 0xff, 0xff, 0xff, 0xff]);
  data[0x1d - 3] = 7;
  data[PUMP_MODE_OFFSET as usize] = PumpMode::from_u8(u8::from(mode)).value();
  let command = 0x14;
  let feature: u8 = 0b000;
  write_to_device(device, command, Some(feature), Some(&data));
  println!("Setting pump mode to {:?}", PumpMode::from_u8(mode));
}

pub fn get_pump_mode(device: &HidDevice) -> PumpMode {
  let measurements = get_measurements(&device);
  for measurement in measurements {
    if measurement.name != "Pump mode" {
      continue;
    }
    return PumpMode::from_str(measurement.value);
  }
  PumpMode::Balanced
}
