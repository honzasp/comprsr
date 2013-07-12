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
}

// TODO: change all the expected/got to computed/read
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
      BadHeaderChecksum(expected, got) =>
        fmt!("Bad header checksum, expected %04x, got %04x",
          expected as uint, got as uint),
      BadDataChecksum(expected, got) =>
        fmt!("Bad data checksum, expected %08x, got %08x",
          expected as uint, got as uint),
      BadDataSize(actual, from_file) =>
        fmt!("Bad data size, decompressed %u bytes, but should get %u bytes",
          actual, from_file),
      ReservedFlagUsed(flag) =>
        fmt!("Reserved flag %u is set on", flag),
    }
  }
}
