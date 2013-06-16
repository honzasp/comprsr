use inflate::error;
use inflate::inflater;

use std::cmp;

pub struct BitBuf {
  priv buf: u32,
  priv bits: uint,
}

impl BitBuf {
  pub fn new() -> BitBuf {
    BitBuf { buf: 0, bits: 0 }
  }

  #[inline]
  fn shift_bits(&mut self, bits: uint) -> u32 {
    let ret = self.buf & !(!0 << bits);
    self.buf = self.buf >> bits;
    self.bits = self.bits - bits;
    ret
  }

  #[inline]
  fn push_byte(&mut self, byte: u8) {
    self.buf = self.buf | (byte as u32 << self.bits);
    self.bits = self.bits + 8;
  }

  #[inline]
  fn clear(&mut self) {
    self.buf = 0;
    self.bits = 0;
  }
}

pub struct BitReader<'self> {
  priv rest_bytes: &'self [u8],
  // invariant: there are less than 8 bits in BitBuf
  priv bit_buf: BitBuf,
}

impl<'self> BitReader<'self> {
  pub fn with_buf<'a>(
    bit_buf: &mut BitBuf,
    chunk: &'a [u8],
    body: &fn(&mut BitReader) -> Option<Result<(),~error::Error>>
  ) -> inflater::Res<&'a [u8]>
  {
    let mut bit_reader = BitReader {
      rest_bytes: chunk,
      bit_buf: *bit_buf,
    };

    match body(&mut bit_reader) {
      None => {
        // TODO: add sanity check
        for bit_reader.rest_bytes.each |&byte| {
          bit_reader.bit_buf.push_byte(byte);
        }
        *bit_buf = bit_reader.bit_buf;
        inflater::ConsumedRes
      },
      Some(Ok(()))   => inflater::FinishedRes(bit_reader.rest_bytes),
      Some(Err(err)) => inflater::ErrorRes(err, bit_reader.rest_bytes),
    }
  }

  pub fn has_bits(&self, bits: uint) -> bool {
    // TODO: add sanity check (bits <= 16)
    if self.rest_bytes.len() >= 2 {
      true
    } else {
      bits <= self.rest_bytes.len() * 8 + self.bit_buf.bits
    }
  }

  pub fn has_bytes(&self, bytes: uint) -> bool {
    // TODO: add sanity check (bit_buf.bits == 0)
    bytes <= self.rest_bytes.len()
  }

  pub fn skip_to_byte(&mut self) {
    // TODO: add sanity check (bit_buf.bits < 8)
    self.bit_buf.clear();
  }

  fn read_bits(&mut self, bits: uint) -> u32 {
    while self.bit_buf.bits < bits {
      self.bit_buf.push_byte(*self.rest_bytes.head());
      self.rest_bytes = self.rest_bytes.tail();
    }

    self.bit_buf.shift_bits(bits)
  }

  pub fn read_bits8(&mut self, bits: uint) -> u8 {
    self.read_bits(bits) as u8
  }

  pub fn read_bits16(&mut self, bits: uint) -> u16 {
    self.read_bits(bits) as u16
  }

  pub fn read_u16(&mut self) -> u16 {
    // TODO: add sanity check (bit_buf.bits == 0)
    let lsb = self.rest_bytes[0];
    let msb = self.rest_bytes[1];
    self.rest_bytes = self.rest_bytes.tailn(2);
    msb as u16 * 256 + lsb as u16
  }

  pub fn read_byte_chunk(&mut self, limit: uint) -> &'self [u8] {
    // TODO: add sanity check (bit_buf.bits == 0)
    let len = cmp::min(limit, self.rest_bytes.len());
    let chunk = self.rest_bytes.slice(0, len);
    let rest = self.rest_bytes.slice(len, self.rest_bytes.len());

    self.rest_bytes = rest;
    chunk
  }
}

#[cfg(test)]
mod test {
  use inflate::bits::*;

  use inflate::inflater;
  use inflate::error;

  #[test]
  fn test_with_buf() {
    let mut n = 0;
    do BitReader::with_buf(&mut BitBuf::new(), &[]) |_| {
      n = n + 1;
      None
    };
    assert_eq!(n, 1);
  }

  #[test]
  fn test_with_buf_carry() {
    let mut bit_buf = BitBuf::new();

    do BitReader::with_buf(&mut bit_buf, &[0b11110010, 0b101001_10])
      |reader|
    {
      reader.read_bits16(10);
      assert!(reader.has_bits(6) && !reader.has_bits(7));
      None
    };

    do BitReader::with_buf(&mut bit_buf, &[0b10_010010])
      |reader|
    {
      assert_eq!(reader.read_bits16(12), 0b010010_101001);
      assert!(reader.has_bits(2) && !reader.has_bits(3));
      None
    };
  }

  #[test]
  fn test_with_buf_carry_byte() {
    let mut bit_buf = BitBuf::new();

    do BitReader::with_buf(&mut bit_buf, &[0b11_110010, 0b10100110])
      |reader|
    {
      reader.read_bits16(6);
      None
    };

    do BitReader::with_buf(&mut bit_buf, &[0b100_10010])
      |reader|
    {
      assert_eq!(reader.read_bits16(15), 0b10010_10100110_11);
      assert!(reader.has_bits(3) && !reader.has_bits(4));
      None
    };
  }

  #[test]
  fn test_with_buf_return() {
    assert_eq!(inflater::ConsumedRes, do BitReader::with_buf(&mut BitBuf::new(), 
      &[0b1101_0010, 0b1110_0100]) |reader|
    {
      reader.read_bits8(6);
      None
    });

    assert_eq!(inflater::FinishedRes(&[0b1110_0100]),
      do BitReader::with_buf(&mut BitBuf::new(), &[0b1101_0010, 0b1110_0100])
        |reader|
      {
        reader.read_bits8(6);
        Some(Ok(()))
      });

    let err = ~error::BadBlockType(4);
    assert_eq!(inflater::ErrorRes(err.clone(), &[0b1110_0100]),
      do BitReader::with_buf(&mut BitBuf::new(), &[0b1101_0010, 0b1110_0100])
        |reader|
      {
        reader.read_bits8(6);
        Some(Err(err.clone()))
      });
  }

  #[test]
  fn test_read_and_has_bits() {
    do BitReader::with_buf(&mut BitBuf::new(), &[0b10001_100, 0b01011_101])
      |reader|
    {
      assert_eq!(reader.read_bits8(3), 0b100);
      assert!(reader.has_bits(13) && !reader.has_bits(14));
      assert_eq!(reader.read_bits8(8), 0b101_10001);
      assert_eq!(reader.read_bits8(5), 0b01011);
      assert!(!reader.has_bits(1));
      None
    };

    do BitReader::with_buf(&mut BitBuf::new(), 
      &[0b10001_100, 0b010_11101, 0b10011101, 0b001_11001]) |reader| 
    {
      assert_eq!(reader.read_bits8(3), 0b100);
      assert_eq!(reader.read_bits16(10), 0b11101_10001);
      assert!(reader.has_bits(16));
      assert_eq!(reader.read_bits16(16), 0b11001_10011101_010);
      assert!(reader.has_bits(3) && !reader.has_bits(4));
      None
    };
  }

  #[test]
  fn test_skip_to_byte() {
    do BitReader::with_buf(&mut BitBuf::new(), &[0b111_01101, 0b01_011100])
      |reader|
    {
      assert_eq!(reader.read_bits8(5), 0b01101);
      reader.skip_to_byte();
      assert_eq!(reader.read_bits8(6), 0b011100);
      None
    };

    do BitReader::with_buf(&mut BitBuf::new(), &[0b11101101, 0b01011100])
      |reader|
    {
      reader.skip_to_byte();
      assert_eq!(reader.read_bits8(8), 0b11101101);
      reader.skip_to_byte();
      assert_eq!(reader.read_bits8(8), 0b01011100);
      reader.skip_to_byte();
      None
    };
  }

  #[test]
  fn test_has_bytes() {
    do BitReader::with_buf(&mut BitBuf::new(),
      &[0b1_1101101, 10, 20, 30]) |reader|
    {
      reader.read_bits8(7);
      reader.skip_to_byte();
      assert!(reader.has_bytes(3) && !reader.has_bytes(4));
      reader.read_bits16(16);
      assert!(reader.has_bytes(1) && !reader.has_bytes(2));
      reader.read_bits8(8);
      assert!(reader.has_bytes(0) && !reader.has_bytes(1));
      None
    };
  }

  #[test]
  fn test_read_u16() {
    do BitReader::with_buf(&mut BitBuf::new(),
      &[10, 0b11101101, 0b11001010, 20, 0b00010100, 0b10011100]) |reader|
    {
      reader.read_bits8(3);
      reader.skip_to_byte();
      assert_eq!(reader.read_u16(), 0b11001010_11101101);
      reader.read_bits16(8);
      assert_eq!(reader.read_u16(), 0b10011100_00010100);
      None
    };
  }

  #[test]
  fn test_read_byte_chunk() {
    do BitReader::with_buf(&mut BitBuf::new(),
      &[2,3,5,7,11,13,17,19,23,29]) |reader|
    {
      reader.read_bits8(5);
      reader.skip_to_byte();
      assert_eq!(reader.read_byte_chunk(6),
        &[3,5,7,11,13,17]);
      assert_eq!(reader.read_byte_chunk(6),
        &[19,23,29]);
      None
    };
  }
}

