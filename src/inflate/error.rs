use std::num::{ToStrRadix};

#[deriving(Clone,Eq)]
pub enum Error {
  BadBlockType(uint),
  BadLitlenCode(uint),
  BadDistCode(uint),
  VerbatimLengthMismatch(u16, u16),
  ReferenceBeforeStart(uint, uint, uint),
  ReferenceOutOfWindow(uint, uint, uint),
}

impl ToStr for Error {
  fn to_str(&self) -> ~str {
    match *self {
      BadBlockType(btype) =>
        fmt!("Bad block type %?", btype),
      BadLitlenCode(code) =>
        fmt!("Bad lit/len code %?", code),
      BadDistCode(code) =>
        fmt!("Bad dist code %?", code),
      VerbatimLengthMismatch(len, nlen) =>
        fmt!("Mismatch between verbatim block length %016s (%?) \
              and its inverse %016s (%?)",
          len.to_str_radix(2), len, nlen.to_str_radix(2), nlen),
      ReferenceBeforeStart(dist, len, out_yet) =>
        fmt!("Reference to distance %? (len %?), only %? output yet",
          dist, len, out_yet),
      ReferenceOutOfWindow(dist, len, win_len) =>
        fmt!("Reference to distance %? (len %?), window only %?",
          dist, len, win_len),
    }
  }
}
