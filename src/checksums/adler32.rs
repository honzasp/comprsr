struct Adler32 {
  priv s1: u32,
  priv s2: u32,
  priv i: uint,
}

impl Adler32 {
  pub fn new() -> Adler32 {
    Adler32 { s1: 1, s2: 0, i: 0 }
  }

  pub fn update(&mut self, chunk: &[u8]) {
    for chunk.each |&b| {
      self.s1 = self.s1 + b as u32;
      self.s2 = self.s2 + self.s1;
      self.i = self.i + 1;

      if self.i >= 5550 {
        self.s1 = self.s1 % 65521;
        self.s2 = self.s2 % 65521;
        self.i = 0;
      }
    }
  }

  pub fn adler32(&self) -> u32 {
    ((self.s2 % 65521) << 16) | (self.s1 % 65521)
  }
}

#[cfg(test)]
mod test {
  use checksums::adler32;

  fn adler32(bytes: &[u8]) -> u32 {
    let mut a32 = adler32::Adler32::new();
    a32.update(bytes);
    a32.adler32()
  }

  #[test]
  fn test_adler32_small() {
    assert_eq!(adler32(&[1, 2, 3]), 0x000d0007);

    assert_eq!(adler32(&[
        38, 101, 228, 50, 170, 180, 36, 50, 248, 165, 
        115, 175, 223, 37, 68, 61, 23, 184, 210, 172
      ]), 0x648709e7);

    {
      let mut bytes = ~[];
      for 100.times {
        bytes.push_all(&[231, 251, 14, 182, 171, 213, 36, 190, 255, 107]);
      }

      assert_eq!(0xdf9884a7, adler32(bytes));
    };
  }

  #[test]
  fn test_adler32_chunked() {
    { // small
      let mut a32 = adler32::Adler32::new();

      let p1 = &[82, 202, 210, 155, 185, 218, 188, 157, 191, 102, 161];
      let p2 = &[246, 246, 148, 94, 231];
      let p3 = &[72, 52, 133, 242, 76, 230, 135];

      for 80.times {
        a32.update(p1);
        a32.update(p2);
      }

      for 60.times {
        a32.update(p3);
      }

      for 50.times {
        a32.update(p2);
        a32.update(p3);
        a32.update(p1);
      }

      assert_eq!(a32.adler32(), 0x24222a52);
    };

    { // large
      let mut a32 = adler32::Adler32::new();

      let p1 = [226, 86, 37, 47, 84, 162, 223, 199, 233];
      let p2 = [132, 117, 82, 213, 92, 17];
      let p3 = [194, 195, 224, 126, 196, 197, 129, 192];

      for 300.times {
        a32.update(p1);
        a32.update(p2);
      }

      for 131.times {
        a32.update(p3);
        a32.update(p1);
      }

      for 54.times {
        a32.update(p2);
      }

      assert_eq!(a32.adler32(), 0x1bd6f6f3);
    };
  }
}
