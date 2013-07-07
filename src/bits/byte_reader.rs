use ByteBuf;

pub struct ByteReader<'self> {
  priv rest_bytes: &'self [u8],
  priv byte_buf: ByteBuf,
}

impl<'self> ByteReader<'self> {
  pub fn with_buf<'a, R>(
    byte_buf: &mut ByteBuf,
    chunk: &'a [u8],
    body: &fn(&mut ByteReader) -> Option<R>
  ) -> Option<(R, &'a [u8])>
  {
    // TODO: is it possible to avoid cloning the byte_buf?

    let mut byte_reader = ByteReader {
        rest_bytes: chunk,
        byte_buf: byte_buf.clone(),
      };

    let opt_x = body(&mut byte_reader);

    let ByteReader {
        rest_bytes,
        byte_buf: new_byte_buf 
      } = byte_reader;

    *byte_buf = new_byte_buf;

    match opt_x {
      Some(x) => {
        assert!(byte_buf.is_empty());
        Some((x, rest_bytes))
      },
      None => {
        byte_buf.push_bytes(rest_bytes);
        None
      },
    }
  }

  pub fn has_bytes(&self, n: uint) -> bool {
    n <= self.rest_bytes.len() + self.byte_buf.byte_count()
  }

  pub fn read_byte(&mut self) -> u8 {
    if !self.byte_buf.is_empty() {
      self.byte_buf.shift_byte()
    } else {
      assert!(self.rest_bytes.len() >= 1);
      let res = self.rest_bytes[0];
      self.rest_bytes = self.rest_bytes.tail();
      res
    }
  }

  pub fn has_some_bytes(&self) -> bool {
    !(self.byte_buf.is_empty() && self.rest_bytes.is_empty())
  }

  pub fn consume_chunk<'a, T>(
    &mut self,
    body: &fn(&'a [u8]) -> (T, Option<&'a [u8]>)
  ) -> T 
  {
    if !self.byte_buf.is_empty() {
      self.byte_buf.consume_buf(body)
    } else {
      let (x, opt_rest) = body(self.rest_bytes);
      self.rest_bytes = match opt_rest {
        Some(rest) => rest,
        None => self.rest_bytes.slice(0, 0),
      };
      x
    }
  }

  pub fn read_be_u32(&mut self) -> u32 {
    assert!(self.has_bytes(4));
    let a = self.read_byte() as u32;
    let b = self.read_byte() as u32;
    let c = self.read_byte() as u32;
    let d = self.read_byte() as u32;

    (a << 24) | (b << 16) | (c << 8) | d
  }

  pub fn read_be_u16(&mut self) -> u16 {
    assert!(self.has_bytes(2));
    let a = self.read_byte() as u16;
    let b = self.read_byte() as u16;

    (a << 8) | b
  }

  pub fn read_le_u32(&mut self) -> u32 {
    assert!(self.has_bytes(4));
    let a = self.read_byte() as u32;
    let b = self.read_byte() as u32;
    let c = self.read_byte() as u32;
    let d = self.read_byte() as u32;

    (d << 24) | (c << 16) | (b << 8) | a
  }

  pub fn read_le_u16(&mut self) -> u16 {
    assert!(self.has_bytes(2));
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
  fn test_with_buf() {
    { // make sure the body is called exactly once
      let mut n = 0;
      do ByteReader::with_buf(&mut ByteBuf::new(), &[]) |_| {
        n = n + 1;
        Some(())
      };
      assert_eq!(n, 1);
    }

    { // the return value
      assert_eq!(
        do ByteReader::with_buf(&mut ByteBuf::new(), &[1, 2]) |_| { Some(()) },
        Some(((), &[1, 2]))
      );
    }
  }

  #[test]
  fn test_with_buf_carry() {
    let mut byte_buf = ByteBuf::new();
    let none: Option<bool> = None;

    do ByteReader::with_buf(&mut byte_buf, &[10, 20, 30])
      |reader|
    {
      reader.read_byte();
      assert!(reader.has_bytes(2) && !reader.has_bytes(3));
      none
    };

    do ByteReader::with_buf(&mut byte_buf, &[40, 50])
      |reader|
    {
      assert!(reader.has_some_bytes());
      assert!(reader.has_bytes(4) && !reader.has_bytes(5));
      none
    };

    let res = do ByteReader::with_buf(&mut byte_buf, &[60, 70, 80])
      |reader|
    {
      assert!(reader.has_some_bytes());
      assert!(reader.has_bytes(7));
      assert_eq!(reader.read_byte(), 20);
      assert_eq!(reader.read_byte(), 30);
      assert_eq!(reader.read_byte(), 40);
      assert_eq!(reader.read_byte(), 50);
      assert_eq!(reader.read_byte(), 60);
      Some(true)
    };

    assert_eq!(res, Some((true, &[70, 80])));

    do ByteReader::with_buf(&mut byte_buf, &[1, 2, 3])
      |reader|
    {
      assert!(reader.has_bytes(3));
      assert!(!reader.has_bytes(4));
      assert_eq!(reader.read_byte(), 1);
      assert_eq!(reader.read_byte(), 2);
      assert_eq!(reader.read_byte(), 3);
      none
    };
  }

  #[test]
  fn test_read_byte() {
    let none: Option<()> = None;

    do ByteReader::with_buf(&mut ByteBuf::new(), &[10, 20, 30, 40, 50])
      |reader|
    {
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

      none
    };
  }

  #[test]
  fn test_consume_chunk() {
    { // consume without remainder
      let mut buf = ByteBuf::new();
      let none: Option<()> = None;

      do ByteReader::with_buf(&mut buf, &[11, 22, 33, 44]) |reader| {
        assert_eq!(reader.read_byte(), 11);
        none
      };

      do ByteReader::with_buf(&mut buf, &[55, 66]) |reader| {
        assert_eq!(reader.read_byte(), 22);
        assert_eq!(reader.read_byte(), 33);
        none
      };

      do ByteReader::with_buf(&mut buf, &[77, 88, 99]) |reader| {
        let mut bytes = ~[];
        while reader.has_some_bytes() {
          do reader.consume_chunk |chunk| {
            bytes.push_all(chunk);
            ((), None)
          };
        }
        assert_eq!(bytes, ~[44, 55, 66, 77, 88, 99]);
        none
      };
    }

    { // consume with remainders
      let mut buf = ByteBuf::new();
      let none: Option<()> = None;

      do ByteReader::with_buf(&mut buf, &[11, 22, 33, 44]) |_reader| {
        none
      };

      do ByteReader::with_buf(&mut buf, &[55, 66, 77]) |_reader| {
        none
      };

      let res = do ByteReader::with_buf(&mut buf, &[88, 99, 111]) |reader| {
        let mut jar: ~[~[u8]] = ~[];
        let mut cookie = ~[];

        while reader.has_some_bytes() {
          // consume 4-byte cookies
          let all_cookies = do reader.consume_chunk |chunk| {
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
        Some('a')
      };

      assert_eq!(res, Some(('a', &[99, 111])));
    }
  }

  #[test]
  fn test_read_big_endian() {
    let none: Option<()> = None;
    do ByteReader::with_buf(&mut ByteBuf::new(), &[
        0xab, 0xcd,
        0xde, 0xad, 0xbe, 0xef,
        0x12, 0x34, 0x56, 0x78,
        0xd2, 0x3c,
      ]) |reader|
    {
      assert_eq!(reader.read_be_u16(), 0xabcd);
      assert_eq!(reader.read_be_u32(), 0xdeadbeef);
      assert_eq!(reader.read_be_u32(), 0x12345678);
      assert_eq!(reader.read_be_u16(), 0xd23c);
      none
    };
  }

  #[test]
  fn test_read_little_endian() {
    let none: Option<()> = None;

    do ByteReader::with_buf(&mut ByteBuf::new(), &[
        0xcd, 0xab,
        0xef, 0xbe, 0xad, 0xde,
        0x78, 0x56, 0x34, 0x12,
        0x3c, 0xd2,
      ]) |reader| 
    {
      assert_eq!(reader.read_le_u16(), 0xabcd);
      assert_eq!(reader.read_le_u32(), 0xdeadbeef);
      assert_eq!(reader.read_le_u32(), 0x12345678);
      assert_eq!(reader.read_le_u16(), 0xd23c);
      none
    };
  }
}

