use inflate;

#[deriving(Clone,Eq)]
pub enum Error {
  InflateError(~inflate::error::Error),
  BadMagicNumber(u16, u16),
  BadCompressionMethod(uint),
  BadHeaderChecksum(u16, u16),
  BadDataChecksum(u32, u32),
  BadDataSize(uint, uint),
  ReservedFlagUsed(uint),
  TrailingExtraBytes(uint),
  ExtraTooLong(uint, uint),
}

impl ToStr for Error {
  pub fn to_str(&self) -> ~str {
    match *self {
      InflateError(ref err) =>
        fmt!("Inflate error: %s", err.to_str()),
      BadMagicNumber(expected, got) =>
        fmt!("Bad magic number, expected %04x, got %04x",
          expected as uint, got as uint),
      BadCompressionMethod(cm) =>
        fmt!("Bad compression method %u", cm),
      BadHeaderChecksum(computed, read) =>
        fmt!("Bad header checksum, computed %04x, but in file is %04x",
          computed as uint, read as uint),
      BadDataChecksum(computed, read) =>
        fmt!("Bad data checksum, decompressed %08x, in trailer %08x",
          computed as uint, read as uint),
      BadDataSize(actual, from_file) =>
        fmt!("Bad data size, decompressed %u bytes, in trailer %u bytes",
          actual, from_file),
      ReservedFlagUsed(flag) =>
        fmt!("Reserved flag %u is set on", flag),
      TrailingExtraBytes(count) =>
        fmt!("Trailing %u bytes of extra field", count),
      ExtraTooLong(remained, requested) =>
        fmt!("An extra subfield too long, only %u bytes remained but %u requested",
          remained, requested),
    }
  }
}
