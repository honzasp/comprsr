use deflate::error::*;

pub enum ZlibError {
  UnknownCompressionMethod(uint),
  FlagsCorrupted,
  PresetDictionaryUsed,
  DeflatingError(~DeflateError),
}

impl ToStr for ZlibError {
  fn to_str(&self) -> ~str {
    match self {
      &UnknownCompressionMethod(id) =>
        fmt!("Unknown compression method %u", id),
      &FlagsCorrupted =>
        fmt!("Header flags are corrupted"),
      &PresetDictionaryUsed =>
        fmt!("A preset dictionary is used"),
      &DeflatingError(~err) =>
        fmt!("Deflate error: %s", err.to_str()),
    }
  }
}


