use ByteBuf;

pub struct ByteReader<'self> {
  priv rest_bytes: &'self [u8],
  priv byte_buf: &'self ByteBuf,
}

impl<'self> ByteReader<'self> {
  pub fn with_buf<'a, R>(
    _byte_buf: &mut ByteBuf,
    _chunk: &'a [u8],
    _body: &fn(&mut ByteReader) -> Option<R>
  ) -> Option<(R, &'a [u8])>
  {
    fail!()
  }

  pub fn has_bytes(&self, _n: uint) -> bool {
    fail!()
  }

  pub fn read_byte(&mut self) -> u8 {
    fail!()
  }

  pub fn has_some_bytes(&self) -> bool {
    fail!()
  }

  pub fn read_chunk(&mut self) -> &'self [u8] {
    fail!()
  }

  pub fn unread_chunk(&mut self, _chunk: &'self [u8]) {
    fail!()
  }

  pub fn read_be_u32(&mut self) -> u32 {
    fail!()
  }

  pub fn read_be_u16(&mut self) -> u16 {
    fail!()
  }

  pub fn read_le_u32(&mut self) -> u32 {
    fail!()
  }

  pub fn read_le_u16(&mut self) -> u16 {
    fail!()
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

    let res = do ByteReader::with_buf(&mut byte_buf, &[])
      |reader|
    {
      assert!(reader.has_some_bytes());
      assert!(reader.has_bytes(4));
      assert_eq!(reader.read_byte(), 20);
      assert_eq!(reader.read_byte(), 30);
      Some(true)
    };

    assert_eq!(res, Some((true, &[40, 50])));

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
  fn test_read_chunk() {
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
        bytes.push_all(reader.read_chunk());
      }
      assert_eq!(bytes, ~[44, 55, 66, 77, 88, 99]);
      none
    };
  }

  #[test]
  fn test_unread_chunk() {
    let mut buf = ByteBuf::new();
    let none: Option<()> = None;

    do ByteReader::with_buf(&mut buf, &[1, 2, 3, 5, 8]) |reader| {
      assert_eq!(reader.read_byte(), 1);
      none
    };

    do ByteReader::with_buf(&mut buf, &[13, 21]) |reader| {
      assert_eq!(reader.read_byte(), 2);
      assert_eq!(reader.read_byte(), 3);
      none
    };

    do ByteReader::with_buf(&mut buf, &[34, 55, 89]) |reader| {
      let mut bytes = ~[];
      while reader.has_some_bytes() {
        bytes.push_all(reader.read_chunk());
      }
      assert_eq!(bytes, ~[5, 8, 13, 21, 34, 55, 89]);
      reader.unread_chunk(&[21, 34, 55, 89]);

      assert!(reader.has_bytes(4));
      assert_eq!(reader.read_byte(), 21);
      assert_eq!(reader.read_byte(), 34);
      none
    };

    do ByteReader::with_buf(&mut buf, &[144, 233]) |reader| {
      assert!(reader.has_bytes(4) && !reader.has_bytes(5));
      assert_eq!(reader.read_byte(), 55);
      assert_eq!(reader.read_byte(), 89);
      assert_eq!(reader.read_byte(), 144);
      assert_eq!(reader.read_byte(), 233);
      assert!(!reader.has_some_bytes());
      none
    };
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

