use rand::Rng;

pub fn random_byte() -> u8 {
  rand::thread_rng().gen_range(1..=31) << 3
}

pub fn crc8(data: &[u8]) -> u8 {
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

pub fn u16le_from(buffer: &[u8], offset: usize) -> u16 {
  u16::from_le_bytes([buffer[offset], buffer[offset + 1]])
}

pub fn byte_to_fraction(value: u8) -> f32 {
  ((value as f32) / 255.0) * 100.0
}

pub fn fraction_of_byte(value: f32) -> u8 {
  (value * 255.0).round() as u8
}
