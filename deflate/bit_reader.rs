pub struct BitReader {
  data: ~[u8],
  byte_pos: uint,
  bit_pos: uint,
  buffer: u32
}

impl BitReader {
  pub fn new(data: ~[u8]) -> ~BitReader {
    let buf = if data.len() >= 2 {
        (data[0] as u32) | ((data[1] as u32) << 8)
      } else if data.len() >= 1 {
        data[0] as u32
      } else {
        0
      };

    ~BitReader { byte_pos: 2, bit_pos: 0, buffer: buf, data: data }
  }

  fn next_byte(&mut self) -> u8 {
    let byte = if self.byte_pos < self.data.len() {
      self.data[self.byte_pos]
    } else {
      0
    };
    self.byte_pos = self.byte_pos + 1;
    byte
  }

  pub fn read_bit(&mut self) -> u8 {
    self.read_bits(1) as u8
  }

  pub fn read_bits(&mut self, len: uint) -> u16 {
    let bits = (self.buffer & !(!0 << len)) as u16;
    self.buffer = self.buffer >> len;
    self.bit_pos = self.bit_pos + len;

    while self.bit_pos >= 8 {
      let next_byte = self.next_byte();
      self.buffer = self.buffer | (next_byte as u32 << (16 - self.bit_pos));
      self.bit_pos = self.bit_pos - 8;
    }

    bits
  }

  pub fn read_rev_bits(&mut self, len: uint) -> u16 {
    let mut bits: u16 = 0;
    for len.times {
      bits = bits << 1 | self.read_bits(1);
    }
    bits
  }

  pub fn read_byte(&mut self) -> u8 {
    self.read_bits(8) as u8
  }

  pub fn flush_byte(&mut self) {
    if self.bit_pos != 0 {
      let next_byte = self.next_byte();
      self.buffer = self.buffer >> (8 - self.bit_pos);
      self.buffer = self.buffer | (next_byte as u32 << 8);
      self.bit_pos = 0;
    }
  }

  pub fn eof(&self) -> bool {
    self.byte_pos - 2 >= self.data.len()
  }

  pub fn debug(&mut self) {
    use core::num::*;
    io::println(fmt!("%u %u %24s", self.byte_pos, self.bit_pos,
      self.buffer.to_str_radix(2)));
  }
}

#[cfg(test)]

mod test {
  use deflate::bit_reader::*;

#[test]
  fn test_read_bit() {
    let mut reader = BitReader::new(~[
      0b0111_1000, 0b1001_1100, 0b0001_1101]);

    assert!(reader.read_bit() == 0);
    assert!(reader.read_bit() == 0);
    assert!(reader.read_bit() == 0);
    assert!(reader.read_bit() == 1);
    assert!(reader.read_bit() == 1);
    assert!(reader.read_bit() == 1);
    assert!(reader.read_bit() == 1);
    assert!(reader.read_bit() == 0);

    assert!(reader.read_bit() == 0);
    assert!(reader.read_bit() == 0);
    assert!(reader.read_bit() == 1);
    assert!(reader.read_bit() == 1);
    assert!(reader.read_bit() == 1);
    assert!(reader.read_bit() == 0);
    assert!(reader.read_bit() == 0);
    assert!(reader.read_bit() == 1);

    assert!(reader.read_bit() == 1);
    assert!(reader.read_bit() == 0);
    assert!(reader.read_bit() == 1);
    assert!(reader.read_bit() == 1);
    assert!(reader.read_bit() == 1);
    assert!(reader.read_bit() == 0);
    assert!(reader.read_bit() == 0);
    assert!(reader.read_bit() == 0);

    for 20.times {
      assert!(reader.read_bit() == 0);
    }
  }

#[test]
  fn test_read_bits() {
    let mut reader = BitReader::new(~[
      0b0111_1000, 0b1001_1100, 0b0001_1101, 0b0101_1110]);

    assert!(reader.read_bits(4) == 0b1000);
    assert!(reader.read_bits(2) == 0b11);
    assert!(reader.read_bits(0) == 0);
    assert!(reader.read_bits(5) == 0b100_01);
    assert!(reader.read_bits(10) == 0b1_1101_1001_1);
    assert!(reader.read_bits(16) == 0b00000_0101_1110_000);
  }

#[test]
  fn test_read_too_many_bits() {
    let mut reader = BitReader::new(~[
      0b0111_1000, 0b1001_1100, 0b0001_1101, 0b0101_1110]);

    reader.read_bits(3);
    reader.read_bits(20);
    assert!(reader.read_bits(6) == 0b1_1110_0);
  }

#[test]
  fn test_flush_byte() {
    let mut reader = BitReader::new(~[
      0b0111_1000, 0b1001_1100, 0b0001_1101, 0b0101_1110, 0b1001_0011]);
    
    reader.read_bits(3);
    reader.flush_byte();
    assert!(reader.read_bits(5) == 0b1_1100);
    reader.flush_byte();
    assert!(reader.read_bits(1) == 0b1);
    reader.flush_byte();
    assert!(reader.read_bits(7) == 0b101_1110);
    reader.flush_byte();
    assert!(reader.read_bits(8) == 0b1001_0011);

    for uint::range(0, 15) |n| {
      reader.flush_byte();
      assert!(reader.read_bits(n) == 0);
    }
  }

#[test]
  fn test_read_byte() {
    let mut reader = BitReader::new(~[
      0b1100_1010, 0b0111_0100, 0b1001_0111, 0b0110_1010]);

    assert_eq!(reader.read_bits(3), 0b010);
    reader.flush_byte();
    assert_eq!(reader.read_bits(8), 0b0111_0100);
    assert_eq!(reader.read_bits(8), 0b1001_0111);
    assert_eq!(reader.read_bits(3), 0b010);
  }

#[test]
  fn test_eof() {
    let mut reader = BitReader::new(~[
      0b1100_1010, 0b0111_0100, 0b1001_0111, 0b0110_1010]);

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

#[test]
  fn test_read_rev_bits() {
    let mut reader = BitReader::new(~[
      0b1100_1010, 0b1001_1110, 0b0101_0001]);

    assert_eq!(reader.read_rev_bits(4), 0b0101);
    assert_eq!(reader.read_rev_bits(4), 0b0011);
    assert_eq!(reader.read_rev_bits(11), 0b0111_1001_100);
    assert_eq!(reader.read_rev_bits(9), 0b0_1010_0000);
  }

#[test]
  fn test_empty_reader() {
    let mut empty_reader = BitReader::new(~[]);

    assert!(empty_reader.read_bits(12) == 0);
    empty_reader.flush_byte();
    assert!(empty_reader.read_bits(5) == 0);
  }
}
