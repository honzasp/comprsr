use ByteBuf;
mod sanity;

pub struct ByteReader<'self> {
  priv rest_bytes: &'self [u8],
  priv byte_buf: ByteBuf,
}

impl<'self> ByteReader<'self> {
  pub fn new<'a>(byte_buf: ByteBuf, chunk: &'a [u8]) -> ByteReader<'a> {
    ByteReader { rest_bytes: chunk, byte_buf: byte_buf }
  }

  pub fn close_to_buf(self) -> ByteBuf {
    let rest_bytes = self.rest_bytes;
    let mut byte_buf = self.byte_buf;
    byte_buf.push_bytes(rest_bytes);
    byte_buf
  }

  pub fn close(self) -> (ByteBuf, &'self [u8]) {
    let ByteReader { rest_bytes, byte_buf } = self;
    (byte_buf, rest_bytes)
  }

  pub fn has_bytes(&self, n: uint) -> bool {
    n <= self.rest_bytes.len() + self.byte_buf.byte_count()
  }

  pub fn read_byte(&mut self) -> u8 {
    if !self.byte_buf.is_empty() {
      self.byte_buf.shift_byte()
    } else {
      sanity!(self.rest_bytes.len() >= 1);
      let res = self.rest_bytes[0];
      self.rest_bytes = self.rest_bytes.tail();
      res
    }
  }

  pub fn has_some_bytes(&self) -> bool {
    !(self.byte_buf.is_empty() && self.rest_bytes.is_empty())
  }

  pub fn consume_chunk<'a, A, T>(
    &mut self,
    arg: A,
    body: &once fn(A, &'a [u8]) -> (T, Option<&'a [u8]>)
  ) -> T 
  {
    if !self.byte_buf.is_empty() {
      self.byte_buf.consume_buf(arg, body)
    } else {
      let (x, opt_rest) = body(arg, self.rest_bytes);
      self.rest_bytes = match opt_rest {
        Some(rest) => rest,
        None => self.rest_bytes.slice(0, 0),
      };
      x
    }
  }

  pub fn read_u32_be(&mut self) -> u32 {
    sanity!(self.has_bytes(4));
    let a = self.read_byte() as u32;
    let b = self.read_byte() as u32;
    let c = self.read_byte() as u32;
    let d = self.read_byte() as u32;

    (a << 24) | (b << 16) | (c << 8) | d
  }

  pub fn read_u16_be(&mut self) -> u16 {
    sanity!(self.has_bytes(2));
    let a = self.read_byte() as u16;
    let b = self.read_byte() as u16;

    (a << 8) | b
  }

  pub fn read_u32_le(&mut self) -> u32 {
    sanity!(self.has_bytes(4));
    let a = self.read_byte() as u32;
    let b = self.read_byte() as u32;
    let c = self.read_byte() as u32;
    let d = self.read_byte() as u32;

    (d << 24) | (c << 16) | (b << 8) | a
  }

  pub fn read_u16_le(&mut self) -> u16 {
    sanity!(self.has_bytes(2));
    let a = self.read_byte() as u16;
    let b = self.read_byte() as u16;

    (b << 8) | a
  }
}

#[cfg(test)]
mod test {
  use ByteReader;
  use ByteBuf;

  #[test]
  fn test_with_buf_carry() {
    let mut byte_buf = ByteBuf::new();

    {
      let mut reader = ByteReader::new(byte_buf, &[10, 20, 30]);
      reader.read_byte();
      assert!(reader.has_bytes(2) && !reader.has_bytes(3));
      byte_buf = reader.close_to_buf();
    };

    {
      let reader = ByteReader::new(byte_buf, &[40, 50]);
      assert!(reader.has_some_bytes());
      assert!(reader.has_bytes(4) && !reader.has_bytes(5));
      byte_buf = reader.close_to_buf();
    };

    {
      let mut reader = ByteReader::new(byte_buf, &[60, 70, 80]);
      assert!(reader.has_some_bytes());
      assert!(reader.has_bytes(7));
      assert_eq!(reader.read_byte(), 20);
      assert_eq!(reader.read_byte(), 30);
      assert_eq!(reader.read_byte(), 40);
      assert_eq!(reader.read_byte(), 50);
      assert_eq!(reader.read_byte(), 60);
      let (_new_buf, rest) = reader.close();
      assert_eq!(rest, &[70, 80]);
    };
  }

  #[test]
  fn test_read_byte() {
    let mut reader = ByteReader::new(ByteBuf::new(), &[10, 20, 30, 40, 50]);
    assert!(reader.has_bytes(2));
    assert_eq!(reader.read_byte(), 10);
    assert_eq!(reader.read_byte(), 20);
    assert!(reader.has_bytes(3));
    assert!(reader.has_some_bytes());
    assert!(!reader.has_bytes(5));
    assert_eq!(reader.read_byte(), 30);
    assert_eq!(reader.read_byte(), 40);
    assert_eq!(reader.read_byte(), 50);
    assert!(!reader.has_bytes(1));
    assert!(reader.has_bytes(0));
  }

  #[test]
  fn test_consume_chunk() {
    { // consume without remainder
      let mut buf = ByteBuf::new();

      {
        let mut reader = ByteReader::new(buf, &[11, 22, 33, 44]);
        assert_eq!(reader.read_byte(), 11);
        buf = reader.close_to_buf();
      };

      {
        let mut reader = ByteReader::new(buf, &[55, 66]);
        assert_eq!(reader.read_byte(), 22);
        assert_eq!(reader.read_byte(), 33);
        buf = reader.close_to_buf();
      };

      {
        let mut reader = ByteReader::new(buf, &[77, 88, 99]);
        let mut bytes = ~[];
        while reader.has_some_bytes() {
          do reader.consume_chunk('a') |arg, chunk| {
            assert_eq!(arg, 'a');
            bytes.push_all(chunk);
            ((), None)
          };
        }
        assert_eq!(bytes, ~[44, 55, 66, 77, 88, 99]);
      };
    }

    { // consume with remainders
      let mut buf = ByteBuf::new();

      {
        let reader = ByteReader::new(buf, &[11, 22, 33, 44]);
        buf = reader.close_to_buf();
      };

      {
        let reader = ByteReader::new(buf, &[55, 66, 77]);
        buf = reader.close_to_buf();
      };

      {
        let mut reader = ByteReader::new(buf, &[88, 99, 111]);
        let mut jar: ~[~[u8]] = ~[];
        let mut cookie = ~[];

        while reader.has_some_bytes() {
          // consume 4-byte cookies
          let all_cookies = do reader.consume_chunk(42) |arg, chunk| {
            assert_eq!(arg, 42);
            let mut rest = chunk;

            while cookie.len() < 4 && !rest.is_empty() {
              cookie.push(rest[0]);
              rest = rest.slice(1, rest.len());
            }

            let opt_rest = if cookie.len() >= 4 {
                jar.push(cookie.clone());
                cookie = ~[];
                Some(rest)
              } else {
                assert!(rest.is_empty());
                None
              };
            let all_cookies = jar.len() >= 2;

            (all_cookies, opt_rest)
          };

          if all_cookies {
            break;
          }
        }
        assert_eq!(jar, ~[~[11, 22, 33, 44], ~[55, 66, 77, 88]]);
      };
    }
  }

  #[test]
  fn test_read_big_endian() {
    let mut reader = ByteReader::new(ByteBuf::new(), &[
        0xab, 0xcd,
        0xde, 0xad, 0xbe, 0xef,
        0x12, 0x34, 0x56, 0x78,
        0xd2, 0x3c,
      ]);
    assert_eq!(reader.read_u16_be(), 0xabcd);
    assert_eq!(reader.read_u32_be(), 0xdeadbeef);
    assert_eq!(reader.read_u32_be(), 0x12345678);
    assert_eq!(reader.read_u16_be(), 0xd23c);
  }

  #[test]
  fn test_read_little_endian() {
    let mut reader = ByteReader::new(ByteBuf::new(), &[
        0xcd, 0xab,
        0xef, 0xbe, 0xad, 0xde,
        0x78, 0x56, 0x34, 0x12,
        0x3c, 0xd2,
      ]);
    
    assert_eq!(reader.read_u16_le(), 0xabcd);
    assert_eq!(reader.read_u32_le(), 0xdeadbeef);
    assert_eq!(reader.read_u32_le(), 0x12345678);
    assert_eq!(reader.read_u16_le(), 0xd23c);
  }
}

