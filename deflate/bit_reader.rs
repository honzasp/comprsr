pub struct BitReader {
  reader: @io::Reader,
  buffer: u32,
  buffer_bits: uint,
}

impl BitReader {
  pub fn new(reader: @io::Reader) -> ~BitReader {
    ~BitReader { reader: reader, buffer: 0, buffer_bits: 0 }
  }

  pub fn read_bit(&mut self) -> u8 {
    self.read_bits(1) as u8
  }

  pub fn read_bits(&mut self, len: uint) -> u16 {
    while self.buffer_bits < len {
      if !self.reader.eof() {
        let byte = self.reader.read_byte() as u32;
        self.buffer = self.buffer | (byte << self.buffer_bits);
        self.buffer_bits = self.buffer_bits + 8;
      } else {
        let bits = self.buffer as u16;
        self.buffer = 0;
        self.buffer_bits = 0;
        return bits;
      }
    }

    let bits = self.buffer & !(!0 << len);
    self.buffer = self.buffer >> len;
    self.buffer_bits = self.buffer_bits - len;
    bits as u16
  }

  pub fn read_rev_bits(&mut self, len: uint) -> u16 {
    let mut bits: u16 = 0;
    for len.times {
      bits = (bits << 1) | self.read_bit() as u16;
    }
    bits
  }

  pub fn read_byte(&mut self) -> u8 {
    if self.buffer_bits % 8 != 0 {
      let discard = self.buffer_bits % 8;
      self.buffer = self.buffer >> discard;
      self.buffer_bits = self.buffer_bits - discard;
    }

    if self.buffer_bits >= 8 {
      let byte = self.buffer_bits & 0xff;
      self.buffer = self.buffer >> 8;
      self.buffer_bits = self.buffer_bits - 8;
      byte as u8
    } else {
      if !self.reader.eof() {
        self.reader.read_byte() as u8
      } else {
        0
      }
    }
  }

  pub fn eof(&self) -> bool {
    self.buffer_bits <= 0 && self.reader.eof()
  }
}

pub fn read_bytes(bytes: &[u8], f: &fn(r: ~BitReader)) {
  let bytes_reader = @io::BytesReader { bytes: bytes, pos: @mut 0 };
  let bit_reader = BitReader::new(bytes_reader as @io::Reader);
  f(bit_reader)
}

#[cfg(test)]
mod test {
  use deflate::bit_reader::{read_bytes};

  #[test]
  fn test_read_bit() {
    do read_bytes(&[0b0111_1000, 0b1001_1100, 0b0001_1101]) |mut reader| {
      assert_eq!(reader.read_bit(), 0);
      assert_eq!(reader.read_bit(), 0);
      assert_eq!(reader.read_bit(), 0);
      assert_eq!(reader.read_bit(), 1);
      assert_eq!(reader.read_bit(), 1);
      assert_eq!(reader.read_bit(), 1);
      assert_eq!(reader.read_bit(), 1);
      assert_eq!(reader.read_bit(), 0);

      assert_eq!(reader.read_bit(), 0);
      assert_eq!(reader.read_bit(), 0);
      assert_eq!(reader.read_bit(), 1);
      assert_eq!(reader.read_bit(), 1);
      assert_eq!(reader.read_bit(), 1);
      assert_eq!(reader.read_bit(), 0);
      assert_eq!(reader.read_bit(), 0);
      assert_eq!(reader.read_bit(), 1);

      assert_eq!(reader.read_bit(), 1);
      assert_eq!(reader.read_bit(), 0);
      assert_eq!(reader.read_bit(), 1);
      assert_eq!(reader.read_bit(), 1);
      assert_eq!(reader.read_bit(), 1);
      assert_eq!(reader.read_bit(), 0);
      assert_eq!(reader.read_bit(), 0);
      assert_eq!(reader.read_bit(), 0);

      for 20.times {
        assert_eq!(reader.read_bit(), 0);
      }
    }
  }

  #[test]
  fn test_read_bits() {
    do read_bytes(&[
      0b0111_1000, 0b1001_1100, 0b0001_1101, 0b0101_1110])
    |mut reader| {
      assert_eq!(reader.read_bits(4), 0b1000);
      assert_eq!(reader.read_bits(2), 0b11);
      assert_eq!(reader.read_bits(0), 0);
      assert_eq!(reader.read_bits(5), 0b100_01);
      assert_eq!(reader.read_bits(10), 0b1_1101_1001_1);
      assert_eq!(reader.read_bits(16), 0b00000_0101_1110_000);
    }
  }

  #[test]
  fn test_read_many_bits() {
    do read_bytes(&[
      0b1100_1010, 0b1001_0110, 0b0111_0100, 0b0100_1101])
    |mut reader| {
      assert_eq!(reader.read_bits(7), 0b100_1010);
      assert_eq!(reader.read_bits(15), 0b11_0100_1001_0110_1);
      assert_eq!(reader.read_bits(6), 0b1101_01);
    }
  }

  #[test]
  fn test_read_too_many_bits() {
    do read_bytes(&[
      0b0111_1000, 0b1001_1100, 0b0001_1101, 0b0101_1110])
    |mut reader| {
      reader.read_bits(3);
      reader.read_bits(20);
      assert_eq!(reader.read_bits(6), 0b1_1110_0);
    }
  }

  #[test]
  fn test_read_byte() {
    do read_bytes(&[
      0b1010_0101, 0b1100_1010, 0b0111_0100, 0b1001_0111, 0b0110_1010])
    |mut reader| {
      assert_eq!(reader.read_bits(11), 0b010_1010_0101);
      assert_eq!(reader.read_byte(), 0b0111_0100);
      assert_eq!(reader.read_byte(), 0b1001_0111);
      assert_eq!(reader.read_bits(3), 0b010);
    }
  }

  #[test]
  fn test_eof() {
    do read_bytes(&[
      0b1100_1010, 0b0111_0100, 0b1001_0111, 0b0110_1010])
    |mut reader| {
      assert!(!reader.eof());
      reader.read_bits(14);
      assert!(!reader.eof());
      reader.read_bits(10);
      assert!(!reader.eof());
      reader.read_bits(7);
      assert!(!reader.eof());
      reader.read_bits(1);
      assert!(reader.eof());
      reader.read_bits(3);
      assert!(reader.eof());
      reader.read_bits(10);
      assert!(reader.eof());
    }
  }

  #[test]
  fn test_read_rev_bits() {
    do read_bytes(&[
      0b1100_1010, 0b1001_1110, 0b0101_0001])
    |mut reader| {
      assert_eq!(reader.read_rev_bits(4), 0b0101);
      assert_eq!(reader.read_rev_bits(4), 0b0011);
      assert_eq!(reader.read_rev_bits(11), 0b0111_1001_100);
      assert_eq!(reader.read_rev_bits(9), 0b0_1010_0000);
    }
  }

  #[test]
  fn test_empty_reader() {
    do read_bytes(&[]) |mut empty_reader| {
      assert_eq!(empty_reader.read_bits(12), 0);
      assert_eq!(empty_reader.read_byte(), 0);
    }
  }
}
