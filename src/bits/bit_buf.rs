// TODO: change assert! to sanity! (and disable those checks in "production" code)

pub struct BitBuf {
  buf: u32,
  bits: uint,
}

impl BitBuf {
  pub fn new() -> BitBuf {
    BitBuf { buf: 0, bits: 0 }
  }

  #[inline]
  pub fn shift_bits(&mut self, bits: uint) -> u32 {
    assert!(bits <= self.bits);
    let ret = self.buf & !(!0 << bits);
    self.buf = self.buf >> bits;
    self.bits = self.bits - bits;
    ret
  }

  #[inline]
  pub fn unshift_bits(&mut self, bits: uint, data: u32) {
    assert!(bits + self.bits <= 32);
    self.buf = (self.buf << bits) | data;
    self.bits = self.bits + bits;
  }

  #[inline]
  pub fn push_byte(&mut self, byte: u8) {
    assert!(self.bits + 8 <= 32);
    self.buf = self.buf | (byte as u32 << self.bits);
    self.bits = self.bits + 8;
  }

  #[inline]
  pub fn clear(&mut self) {
    self.buf = 0;
    self.bits = 0;
  }
}

// TODO: add tests?
