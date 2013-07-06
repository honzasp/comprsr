pub struct ByteBuf {
  buf: ~[u8],
}

impl ByteBuf {
  pub fn new() -> ByteBuf {
    ByteBuf { buf: ~[] }
  }
}
