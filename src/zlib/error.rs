use inflate::error;

#[deriving(Clone,Eq)]
pub enum Error {
  BadCompressionMethod(uint),
  WindowTooLong(uint),
  BadHeaderChecksum(u8, u8),
  BadDataChecksum(u32, u32),
  DictionaryUsed(),
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
        fmt!("Bad header: 0x%02x 0x%02x", cmf as uint, flg as uint),
      BadDataChecksum(expected, got) =>
        fmt!("Bad Adler32 checksum of the data, \
            computed 0x%08x but in file there is 0x%08x",
          expected as uint, got as uint),
      DictionaryUsed =>
        fmt!("Preset dictionary used"),
      InflateError(ref err) =>
        fmt!("Inflate error: %s", err.to_str()),
    }
  }
}
