use std::num::{ToStrRadix};

#[deriving(Clone,Eq)]
pub enum Error {
  BadBlockType(uint),
  BadLitlenCode(uint),
  BadDistCode(uint),
  BadMetaCode(uint),
  VerbatimLengthMismatch(u16, u16),
  ReferenceBeforeStart(uint, uint, uint),
  ReferenceOutOfWindow(uint, uint, uint),
  MetaCopyAtStart(),
  MetaRepeatTooLong(u8, uint, uint),
  TooManyHuffCodesError(uint),
}

impl ToStr for Error {
  fn to_str(&self) -> ~str {
    match *self {
      BadBlockType(btype) =>
        fmt!("Bad block type %u", btype),
      BadLitlenCode(code) =>
        fmt!("Bad lit/len code %u", code),
      BadDistCode(code) =>
        fmt!("Bad dist code %u", code),
      BadMetaCode(code) =>
        fmt!("Bad meta code %u", code),
      VerbatimLengthMismatch(len, nlen) =>
        fmt!("Mismatch between verbatim block length %016s (%u) \
              and its inverse %016s (%u)",
          len.to_str_radix(2), len as uint, nlen.to_str_radix(2), nlen as uint),
      ReferenceBeforeStart(dist, len, out_yet) =>
        fmt!("Reference to distance %u (len %u), only %u output yet",
          dist, len, out_yet),
      ReferenceOutOfWindow(dist, len, win_len) =>
        fmt!("Reference to distance %u (len %u), window only %u",
          dist, len, win_len),
      MetaCopyAtStart =>
        fmt!("Meta copy code at start"),
      MetaRepeatTooLong(len_to_repeat, repeat_count, max_repeat_count) =>
        fmt!("Meta code repeating %u had length %u, maximum %u",
          len_to_repeat as uint, repeat_count, max_repeat_count),
      TooManyHuffCodesError(code_len) =>
        fmt!("Too many %u-bit huffman codes", code_len),
    }
  }
}
