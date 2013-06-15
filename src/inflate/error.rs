#[deriving(Clone)]
pub enum Error {
  BadBlockType(u8),
}

impl ToStr for Error {
  fn to_str(&self) -> ~str {
    match *self {
      BadBlockType(btype) =>
        fmt!("Bad block type %s", btype.to_str()),
    }
  }
}
