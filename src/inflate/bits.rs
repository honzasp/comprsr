pub struct BitReader<'self> {
  priv x: &'self uint,
}

pub struct BitBuf {
  priv x: uint,
}

impl<'self> BitReader<'self> {
  pub fn new<'a>(bit_buf: &BitBuf, chunk: &'a [u8]) -> BitReader<'a> {
    fail!(~"bit reader unimplemented")
  }

  pub fn unconsumed_bytes<'a>(&self, chunk: &'a [u8]) -> &'a [u8] {
    fail!(~"unconsumed bytes unimplemented")
  }

  pub fn has_bits(&self, bits: uint) -> bool {
    fail!()
  }

  pub fn has_bytes(&self, bytes: uint) -> bool {
    fail!()
  }

  pub fn skip_to_byte(&mut self) {
    fail!()
  }

  pub fn read_bits8(&mut self, bits: uint) -> u8 {
    fail!()
  }

  pub fn read_bits16(&mut self, bits: uint) -> u16 {
    fail!()
  }

  pub fn read_u16(&mut self) -> u16 {
    fail!()
  }

  pub fn read_byte_chunk(&mut self, limit: uint) -> &'self [u8] {
    fail!()
  }

  pub fn rest_bit_buf(self) -> BitBuf {
    fail!()
  }
}

impl BitBuf {
  pub fn new() -> BitBuf {
    fail!()
  }
}
