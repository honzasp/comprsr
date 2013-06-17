use std::num::{ToStrRadix};

#[deriving(Clone,Eq)]
pub enum Error {
  BadBlockType(u8),
  VerbatimLengthMismatch(u16, u16),
  DistanceTooLong(uint, uint),
}

impl ToStr for Error {
  fn to_str(&self) -> ~str {
    match *self {
      BadBlockType(btype) =>
        fmt!("Bad block type %s", btype.to_str()),
      VerbatimLengthMismatch(len, nlen) =>
        fmt!("Mismatch between verbatim block length %016s (%s) \
              and its inverse %016s (%s)",
          len.to_str_radix(2), len.to_str(), nlen.to_str_radix(2), nlen.to_str()),
      DistanceTooLong(pos, dist) =>
        fmt!("Distance %u on position %u is too long", dist, pos),
    }
  }
}
