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
    false
  }

  pub fn shift_bits8(&mut self, bits: uint) -> u8 {
    fail!()
  }

  pub fn shift_bits16(&mut self, bits: uint) -> u16 {
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
