struct Adler32 {
  s1: uint,
  s2: uint,
  ss: uint
}

pub struct Adler32Writer {
  writer: @io::Writer,
  adler32: @mut Adler32,
}

impl Adler32Writer {
  pub fn new(writer: @io::Writer) -> @Adler32Writer {
    @Adler32Writer {
      writer: writer,
      adler32: @mut Adler32 { s1: 1, s2: 0, ss: 0 }
    }
  }

  pub fn adler32(&self) -> u32 {
    self.adler32.adler32()
  }
}

impl Adler32 {
  fn adler32(&self) -> u32 {
    let s1 = self.s1 % 65521;
    let s2 = self.s2 % 65521;

    (s2 as u32 << 16) | s1 as u32
  }

  fn add_byte(&mut self, byte: u8) {
    self.s1 = self.s1 + byte as uint;
    self.s2 = self.s2 + self.s1;
    self.ss = self.ss + 1;

    if self.ss > 5500 {
      self.s1 = self.s1 % 65521;
      self.s2 = self.s2 % 65521;
      self.ss = 0;
    }
  }
}

impl io::Writer for Adler32Writer {
  fn write(&self, v: &[u8]) {
    for v.each |&b| {
      self.adler32.add_byte(b);
    }
    self.writer.write(v)
  }

  fn seek(&self, s: int, t: io::SeekStyle) {
    self.writer.seek(s, t)
  }

  fn tell(&self) -> uint {
    self.writer.tell()
  }

  fn flush(&self) -> int {
    self.writer.flush()
  }

  fn get_type(&self) -> io::WriterType {
    self.writer.get_type()
  }
}

#[cfg(test)]
mod test {
  use zlib::adler32_writer::{Adler32Writer};

  #[test]
  fn test_checksum() {
    do io::with_bytes_writer |dummy_w| {
      let w1 = Adler32Writer::new(dummy_w);
      w1.write(&[3,5,7]);
      assert_eq!(w1.adler32(), 16+(29<<16));

      let w2 = Adler32Writer::new(dummy_w);
      for 2000.times {
        w2.write(&[3,5,7]);
      }
      assert_eq!(w2.adler32(), 30001+(52667<<16))
    };
  }
}
