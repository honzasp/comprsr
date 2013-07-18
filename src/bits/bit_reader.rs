use std::cmp;
use BitBuf;
mod sanity;

pub struct BitReader<'self> {
  priv rest_bytes: &'self [u8],
  priv bit_buf: BitBuf,
}

// TODO: once it's possible, rename 'self to 'a (the lifetime 'self *does not* refer to the life
// time of the object, it can be longer! (otherwise the method `.close()` would have no sense, as it
// destroys `self`, but returns `&'self`)
impl<'self> BitReader<'self> {
  pub fn new<'a>(bit_buf: BitBuf, chunk: &'a [u8]) -> BitReader<'a> {
    BitReader { rest_bytes: chunk, bit_buf: bit_buf }
  }

  pub fn close_to_buf(self) -> BitBuf {
    let rest_bytes = self.rest_bytes;
    let mut bit_buf = self.bit_buf;

    sanity!(rest_bytes.len() * 8 + bit_buf.bits <= 16);
    for rest_bytes.iter().advance |&byte| {
      bit_buf.push_byte(byte);
    }
    bit_buf
  }

  pub fn close(self) -> (BitBuf, &'self [u8]) {
    let BitReader { bit_buf, rest_bytes } = self;
    (bit_buf, rest_bytes)
  }

  pub fn has_bits(&self, bits: uint) -> bool {
    sanity!(bits <= 16);
    if self.rest_bytes.len() >= 2 {
      true
    } else {
      bits <= self.rest_bytes.len() * 8 + self.bit_buf.bits
    }
  }

  pub fn has_bytes(&self, bytes: uint) -> bool {
    bytes <= self.bit_buf.bits / 8 + self.rest_bytes.len()
  }

  pub fn skip_to_byte(&mut self) {
    sanity!(self.bit_buf.bits < 8);
    self.bit_buf.clear();
  }

  priv fn read_bits(&mut self, bits: uint) -> u32 {
    sanity!(bits <= 16);
    while self.bit_buf.bits < bits {
      self.bit_buf.push_byte(*self.rest_bytes.head());
      self.rest_bytes = self.rest_bytes.tail();
    }

    self.bit_buf.shift_bits(bits)
  }

  pub fn read_bits8(&mut self, bits: uint) -> u8 {
    sanity!(bits <= 8);
    self.read_bits(bits) as u8
  }

  pub fn read_bits16(&mut self, bits: uint) -> u16 {
    sanity!(bits <= 16);
    self.read_bits(bits) as u16
  }

  pub fn read_rev_bits8(&mut self, bits: uint) -> u8 {
    // TODO: this could surely be optimized
    sanity!(bits <= 8);
    let mut res: u8 = 0;
    for bits.times {
      let bit = self.read_bits8(1);
      res = (res << 1) | bit;
    }
    res
  }

  pub fn unread_bits8(&mut self, bits: uint, data: u8) {
    sanity!(bits <= 8);
    self.bit_buf.unshift_bits(bits, data as u32);
  }

  pub fn unread_bits16(&mut self, bits: uint, data: u16) {
    sanity!(bits <= 16);
    self.bit_buf.unshift_bits(bits, data as u32);
  }

  pub fn read_u16(&mut self) -> u16 {
    self.read_bits16(16)
  }

  pub fn read_byte_chunk(&mut self, limit: uint) -> &'self [u8] {
    sanity!(self.bit_buf.bits == 0);
    let len = cmp::min(limit, self.rest_bytes.len());
    let chunk = self.rest_bytes.slice(0, len);
    let rest = self.rest_bytes.slice(len, self.rest_bytes.len());

    self.rest_bytes = rest;
    chunk
  }
}

#[cfg(test)]
mod test {
  use extra::test;
  use std::rand;
  use std::vec;

  use BitReader;
  use BitBuf;

  #[test]
  fn test_with_buf_carry() {
    let mut bit_buf = BitBuf::new();

    {
      let mut reader = BitReader::new(bit_buf, &[0b11110010, 0b101001_10]);
      reader.read_bits16(10);
      assert!(reader.has_bits(6) && !reader.has_bits(7));

      bit_buf = reader.close_to_buf();
    }

    {
      let mut reader = BitReader::new(bit_buf, &[0b10_010010]);
      assert_eq!(reader.read_bits16(12), 0b010010_101001);
      assert!(reader.has_bits(2) && !reader.has_bits(3));
    }
  }

  #[test]
  fn test_with_buf_carry_byte() {
    let mut bit_buf = BitBuf::new();

    {
      let mut reader = BitReader::new(bit_buf, &[0b11_110010, 0b10100110]);
      reader.read_bits16(6);
      bit_buf = reader.close_to_buf();
    };

    {
      let mut reader = BitReader::new(bit_buf, &[0b100_10010]);
      assert_eq!(reader.read_bits16(15), 0b10010_10100110_11);
      assert!(reader.has_bits(3) && !reader.has_bits(4));
    };
  }

  #[test]
  fn test_with_buf_many_carries() {
    let mut bit_buf = BitBuf::new();

    {
      let mut reader = BitReader::new(bit_buf, 
        &[0b11001_010, 0b111010_11, 0b01000110]);
      assert_eq!(reader.read_bits8(3), 0b010);
      assert_eq!(reader.read_bits8(7), 0b11_11001);
      bit_buf = reader.close_to_buf();
    };

    {
      let mut reader = BitReader::new(bit_buf,
        &[0b1001_0111, 0b0011_1011]);
      assert_eq!(reader.read_bits8(6), 0b111010);
      assert_eq!(reader.read_bits16(12), 0b0111_01000110);
      assert_eq!(reader.read_bits8(8), 0b1011_1001);
    };
  }

  #[test]
  fn test_read_and_has_bits() {
    {
      let mut reader = BitReader::new(BitBuf::new(),
        &[0b10001_100, 0b01011_101]);
      assert_eq!(reader.read_bits8(3), 0b100);
      assert!(reader.has_bits(13) && !reader.has_bits(14));
      assert_eq!(reader.read_bits8(8), 0b101_10001);
      assert_eq!(reader.read_bits8(5), 0b01011);
      assert!(!reader.has_bits(1));
    };

    {
      let mut reader = BitReader::new(BitBuf::new(),
        &[0b10001_100, 0b010_11101, 0b10011101, 0b001_11001]);
      assert_eq!(reader.read_bits8(3), 0b100);
      assert_eq!(reader.read_bits16(10), 0b11101_10001);
      assert!(reader.has_bits(16));
      assert_eq!(reader.read_bits16(16), 0b11001_10011101_010);
      assert!(reader.has_bits(3) && !reader.has_bits(4));
    };
  }

  #[test]
  fn test_skip_to_byte() {
    {
      let mut reader = BitReader::new(BitBuf::new(),
        &[0b111_01101, 0b01_011100]);
      assert_eq!(reader.read_bits8(5), 0b01101);
      reader.skip_to_byte();
      assert_eq!(reader.read_bits8(6), 0b011100);
    };

    {
      let mut reader = BitReader::new(BitBuf::new(), &[0b11101101, 0b01011100]);
      reader.skip_to_byte();
      assert_eq!(reader.read_bits8(8), 0b11101101);
      reader.skip_to_byte();
      assert_eq!(reader.read_bits8(8), 0b01011100);
      reader.skip_to_byte();
    };
  }

  #[test]
  fn test_has_bytes() {
    let mut reader = BitReader::new(BitBuf::new(),
      &[0b1_1101101, 10, 20, 30]);
    reader.read_bits8(7);
    reader.skip_to_byte();
    assert!(reader.has_bytes(3) && !reader.has_bytes(4));
    reader.read_bits16(16);
    assert!(reader.has_bytes(1) && !reader.has_bytes(2));
    reader.read_bits8(8);
    assert!(reader.has_bytes(0) && !reader.has_bytes(1));
  }

  #[test]
  fn test_read_u16() {
    let mut reader = BitReader::new(BitBuf::new(),
      &[10, 0b11101101, 0b11001010, 20, 0b00010100, 0b10011100]);
    reader.read_bits8(3);
    reader.skip_to_byte();
    assert_eq!(reader.read_u16(), 0b11001010_11101101);
    reader.read_bits16(8);
    assert_eq!(reader.read_u16(), 0b10011100_00010100);
  }

  #[test]
  fn test_read_byte_chunk() {
    let mut reader = BitReader::new(BitBuf::new(),
      &[2,3,5,7,11,13,17,19,23,29]);
    reader.read_bits8(5);
    reader.skip_to_byte();
    assert_eq!(reader.read_byte_chunk(6), &[3,5,7,11,13,17]);
    assert_eq!(reader.read_byte_chunk(6), &[19,23,29]);
  }

  #[test]
  fn test_read_rev_bits() {
    let mut reader = BitReader::new(BitBuf::new(),
      &[0b1001_0111, 0b10100_010]);
    assert_eq!(reader.read_rev_bits8(4), 0b1110);
    assert_eq!(reader.read_rev_bits8(7), 0b1001_010);
  }

  #[test]
  fn test_unread_bits() {
    let mut bit_buf = BitBuf::new();

    {
      let mut reader = BitReader::new(bit_buf,
        &[0b11_01_0001, 0b01101_110]);
      assert_eq!(reader.read_bits8(4), 0b0001);
      reader.unread_bits8(4, 0b0001);
      assert_eq!(reader.read_bits8(6), 0b01_0001);
      reader.unread_bits8(4, 0b01_00);
      assert_eq!(reader.read_bits16(9), 0b110_11_01_00);
      bit_buf = reader.close_to_buf();
    };

    {
      let mut reader = BitReader::new(bit_buf,
        &[0b1_10_01110, 0b10_100110, 0b001100_11]);
      assert_eq!(reader.read_bits16(10), 0b01110_01101);
      reader.unread_bits16(10, 0b01110_01101);
      assert_eq!(reader.read_bits16(12), 0b10_01110_01101);
      reader.unread_bits16(3, 0b100);
      assert_eq!(reader.read_bits16(10), 0b100110_1100);
    };
  }

  #[bench]
  fn bench_bits(b: &mut test::BenchHarness) {
    let bytes = vec::from_fn(850, |_| rand::random());

    do b.iter {
      let mut reader = BitReader::new(BitBuf::new(), bytes);

      for 100.times {
        assert!(reader.has_bits(5));
        reader.read_bits8(5);
      }

      for 200.times {
        assert!(reader.has_bits(13));
        reader.read_bits16(13);
      }

      for 1000.times {
        assert!(reader.has_bits(1));
        reader.read_bits8(1);
      }

      for 300.times {
        assert!(reader.has_bits(5));
        reader.read_bits8(3);
        reader.read_bits8(2);
      }

      for 200.times {
        assert!(reader.has_bits(5));
        let x = reader.read_bits8(5);
        reader.unread_bits8(5, x);
        assert!(reader.has_bits(13));
        let y = reader.read_bits16(13);
        reader.unread_bits16(13, y);
      }

      for 100.times {
        assert!(reader.has_bits(9));
        reader.read_bits16(9);
      }
    };
  }

  #[bench]
  fn bench_rev_bits(b: &mut test::BenchHarness) {
    let bytes = vec::from_fn(850, |_| rand::random());

    do b.iter {
      let mut reader = BitReader::new(BitBuf::new(), bytes);

      for 400.times {
        for (&[3u, 1, 6, 4, 1, 1]).iter().advance |&b| {
          assert!(reader.has_bits(b));
          reader.read_rev_bits8(b);
        }
      }
    };
  }

  #[bench]
  fn bench_bytes(b: &mut test::BenchHarness) {
    let bytes = vec::from_fn(12_345, |_| rand::random());

    do b.iter {
      let mut reader = BitReader::new(BitBuf::new(), bytes);
      reader.skip_to_byte();
      while reader.has_bytes(4) {
        reader.read_byte_chunk(32);
      }
    };
  }
}
