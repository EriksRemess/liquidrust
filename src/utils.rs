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
