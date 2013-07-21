mod sanity;

#[deriving(Clone)]
pub struct ByteBuf {
  pub buf: ~[u8],
}

impl ByteBuf {
  pub fn new() -> ByteBuf {
    ByteBuf { buf: ~[] }
  }

  pub fn is_empty(&self) -> bool {
    self.buf.is_empty()
  }

  pub fn byte_count(&self) -> uint {
    self.buf.len()
  }

  pub fn shift_byte(&mut self) -> u8 {
    sanity!(self.buf.len() > 0);
    self.buf.shift()
  }

  pub fn push_bytes(&mut self, bytes: &[u8]) {
    self.buf.push_all(bytes);
  }

  pub fn consume_buf<'a, A, T>(
    &mut self,
    arg: A,
    body: &once fn(A, &'a [u8]) -> (T, Option<&'a [u8]>)
  ) -> T {
    let (x, new_buf) = 
      match body(arg, self.buf) {
        (x, None)       => (x, ~[]),
        (x, Some(rest)) => (x, rest.to_owned()),
      };
    self.buf = new_buf;
    x
  }
}
