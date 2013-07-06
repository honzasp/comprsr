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
