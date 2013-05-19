use deflate::error::*;

pub enum ZlibError {
  MissingHeaderError,
  UnknownCompressionMethodError(uint),
  FlagsCorruptedError,
  PresetDictionaryUsedError,
  DeflatingError(~DeflateError),
  ChecksumMismatchError(u32, u32),
  MissingChecksumError(u32),
}

impl ToStr for ZlibError {
  fn to_str(&self) -> ~str {
    match self {
      &MissingHeaderError =>
        fmt!("Header bytes are missing"),
      &UnknownCompressionMethodError(id) =>
        fmt!("Unknown compression method %u", id),
      &FlagsCorruptedError =>
        fmt!("Header flags are corrupted"),
      &PresetDictionaryUsedError =>
        fmt!("A preset dictionary is used"),
      &DeflatingError(~err) =>
        fmt!("Deflate error: %s", err.to_str()),
      &ChecksumMismatchError(expected, got) =>
        fmt!("Checksum error: expected %u, got %u",
          expected as uint, got as uint),
      &MissingChecksumError(expected) =>
        fmt!("Checksum error: expected %u, found EOF", expected as uint),
    }
  }
}


