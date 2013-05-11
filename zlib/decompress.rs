use zlib::error::*;
use deflate_decompress = deflate::decompress::decompress;
use deflate::bit_reader::{BitReader};

pub fn decompress(in: ~[u8]) -> Result<~[u8],~ZlibError> {
  let cmf = in[0];
  let flg = in[1];

  let cm = (cmf & 0b1111) as uint;
  let fdict = (flg & 0b100000) != 0;

  if cm != 8 {
    Err(~UnknownCompressionMethod(cm))
  } else if fdict {
    Err(~PresetDictionaryUsed)
  } else if (cmf as uint * 256 + flg as uint) % 31 != 0 {
    Err(~FlagsCorrupted)
  } else {
    let mut bit_reader = BitReader::new(vec::from_slice(in.slice(2, in.len() - 4)));
    match deflate_decompress(bit_reader) {
      Ok(data) => Ok(data),
      Err(deflate_err) => Err(~DeflatingError(deflate_err))
    }
  }
}

#[cfg(test)]
mod test {
  use zlib::decompress::{decompress};
  use zlib::error::*;
  use deflate::error::*;

  #[test]
  fn test_decompress() {
    let bytes1 = ~[120,156,99,98,102,101,231,230,5,0,0,109,0,42];
    assert_eq!(decompress(bytes1).unwrap(), ~[2,3,5,7,11,13]);
  }

  #[test]
  fn test_decompress_errors() {
    /* unknown compression method */
    let bytes1 = ~[0b0000_0101, 0];
    match decompress(bytes1) {
      Err(~UnknownCompressionMethod(5)) => { },
      Err(err) => fail!(fmt!("unexpected %s", err.to_str())),
      Ok(_) => fail!(~"expected an error")
    }

    /* FCHECK doesn't match */
    let bytes2 = ~[0b0111_1000, 0b10_0_11101];
    match decompress(bytes2) {
      Err(~FlagsCorrupted) => { },
      Err(err) => fail!(fmt!("unexpected %s", err.to_str())),
      Ok(_) => fail!(~"expected an error")
    }
    
    /* FDICT is on */
    let bytes3 = ~[0b0111_1000, 0b10_1_00110];
    match decompress(bytes3) {
      Err(~PresetDictionaryUsed) => { },
      Err(err) => fail!(fmt!("unexpected %s", err.to_str())),
      Ok(_) => fail!(~"expected an error")
    }
  }

  #[test]
  fn test_decompress_deflate_errors() {
    /* bad deflate block */
    let bytes1 = ~[120,156,0b110,0,0,0,0,0];
    match decompress(bytes1) {
      Err(~DeflatingError(~BadBlockType)) => { },
      Err(err) => fail!(fmt!("unexpected %s", err.to_str())),
      Ok(_) => fail!(~"expected an error")
    }
  }
}
