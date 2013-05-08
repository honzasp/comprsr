use core::num::*;

pub enum DeflateError {
  TooManyHuffCodesError(uint),
  MissingHuffCodesError(uint),
  LengthMismatchError(u16, u16),
  UnexpectedEOFError,
  DistanceTooLong(uint, uint),
  BadLengthCode(u16),
  BadDistCode(u16),
  MetaRepeatAtStart,
  MetaRepeatTooLong(uint, uint),
  MetaUndefinedCode,
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
      DistanceTooLong(pos, dist) =>
        fmt!("Distance %u on position %u is too long", dist, pos),
      BadLengthCode(c) =>
        fmt!("Bad length code %u", c as uint),
      BadDistCode(c) =>
        fmt!("Bad distance code %u", c as uint),
      MetaRepeatAtStart =>
        fmt!("Meta repeat symbol (16) at the start of the sequence"),
      MetaRepeatTooLong(actual, max) =>
        fmt!("Meta repeat too long (%u, maximum was %u)", actual, max),
      MetaUndefinedCode =>
        fmt!("Undefined meta code"),
    }
  }
}

