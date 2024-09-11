use crate::hid::write_to_device;
use crate::pump;
use crate::utils::fraction_of_byte;
use hidapi::HidDevice;

const FAN_MODE_OFFSETS: [u8; 2] = [0x0b - 3, 0x11 - 3];
const FAN_DUTY_OFFSETS: [u8; 2] = [0x0b + 2, 0x11 + 2];

#[derive(Debug, Clone, Copy)]
pub enum FanMode {
  // CustomProfile = 0x0,
  // CustomProfileWithExternalSensor = 0x1,
  FixedDuty = 0x2,
  // FixedRPM = 0x4,
}

impl FanMode {
  pub fn value(&self) -> u8 {
    *self as u8
  }
}

pub fn set_fan_mode(device: &HidDevice, percentage: u32) {
  println!("Setting fan speed to {percentage}%");
  let pump_mode = pump::get_pump_mode(&device);
  let mut data = [0u8; 60];
  data[0..8].copy_from_slice(&[0x0, 0xff, 0x5, 0xff, 0xff, 0xff, 0xff, 0xff]);
  data[0x1d - 3] = 7;
  let mode = FanMode::FixedDuty;
  let duty = fraction_of_byte(percentage as f32 / 100.0);
  for i in 0..2 {
    data[FAN_MODE_OFFSETS[i] as usize] = mode.value();
    data[FAN_DUTY_OFFSETS[i] as usize] = duty;
  }
  data[pump::PUMP_MODE_OFFSET as usize] = pump_mode.value();
  let command = 0x14;
  let feature: u8 = 0b000;
  write_to_device(device, command, Some(feature), Some(&data));
}
