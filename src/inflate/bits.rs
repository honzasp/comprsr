use inflate::error;
use inflate::inflater;

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
}

pub struct BitReader<'self> {
  priv rest_bytes: &'self [u8],
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
    // TODO: add sanity check
    if self.rest_bytes.len() >= 2 {
      true
    } else {
      bits <= self.rest_bytes.len() * 8 + self.bit_buf.bits
    }
  }

  pub fn has_bytes(&self, bytes: uint) -> bool {
    // TODO: add sanity check
    fail!()
  }

  pub fn skip_to_byte(&mut self) {
    fail!()
  }

  pub fn read_bits(&mut self, bits: uint) -> u32 {
    while self.bit_buf.bits < bits {
      self.bit_buf.push_byte(self.rest_bytes[0]);
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
    // TODO: add sanity check
    fail!()
  }

  pub fn read_byte_chunk(&mut self, limit: uint) -> &'self [u8] {
    fail!()
  }

  pub fn rest_bit_buf(self) -> BitBuf {
    fail!()
  }
}

#[cfg(test)]
mod test {
  use inflate::bits::*;

  use inflate::inflater;
  use inflate::error;

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
}

