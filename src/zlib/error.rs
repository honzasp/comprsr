use std::num::{ToStrRadix};
use inflate::error;

#[deriving(Clone,Eq)]
pub enum Error {
  BadCompressionMethod(uint),
  WindowTooLong(uint),
  BadHeaderChecksum(u8, u8),
  DictionaryUsed,
  InflateError(~error::Error),
}

impl ToStr for Error {
  fn to_str(&self) -> ~str {
    match *self {
      BadCompressionMethod(cm) =>
        fmt!("Bad compression method: %u", cm),
      WindowTooLong(size) =>
        fmt!("Window of 2^%u bytes (%u kb) is too long", size, size / 1024),
      BadHeaderChecksum(cmf, flg) =>
        fmt!("Bad header: 0x%02s 0x%02s",
          cmf.to_str_radix(16),
          flg.to_str_radix(16)),
      DictionaryUsed =>
        fmt!("Preset dictionary used"),
      InflateError(ref err) =>
        fmt!("Inflate error: %?", err),
    }
  }
}