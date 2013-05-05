use core::num::*;

pub enum DeflateError {
  TooManyHuffCodesError(uint),
  MissingHuffCodesError(uint),
  LengthMismatchError(u16, u16),
  UnexpectedEOFError,
  BadLengthCode(u16),
  BadDistCode(u16)
}

impl ToStr for DeflateError {
  fn to_str(&self) -> ~str {
    match *self {
      TooManyHuffCodesError(c) =>
        fmt!("Too many Huffman codes with length %u", c),
      MissingHuffCodesError(c) =>
        fmt!("There are missing Huffman codes with length %u", c),
      LengthMismatchError(len, nlen) =>
        fmt!("Mismatch between length %016s and its inverse %016s",
          len.to_str_radix(2), nlen.to_str_radix(2)),
      UnexpectedEOFError =>
        fmt!("Unexpected end of input"),
      BadLengthCode(c) =>
        fmt!("Bad length code %u", c as uint),
      BadDistCode(c) =>
        fmt!("Bad distance code %u", c as uint)
    }
  }
}

