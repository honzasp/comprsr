use zlib::error::*;
use deflate_decompress = deflate::decompress::decompress;
use deflate::bit_reader::{BitReader};
use deflate::output::{Output};

pub fn decompress(in: @io::Reader, out: @io::Writer) 
  -> Option<~ZlibError> 
{
  let cmf = in.read_byte();
  let flg = in.read_byte();

  let cm = (cmf & 0b1111) as uint;
  let fdict = (flg & 0b100000) != 0;

  if cm != 8 {
    Some(~UnknownCompressionMethod(cm))
  } else if fdict {
    Some(~PresetDictionaryUsed)
  } else if (cmf as uint * 256 + flg as uint) % 31 != 0 {
    Some(~FlagsCorrupted)
  } else {
    let mut bit_reader = BitReader::new(in);
    let mut output = Output::new(out);
    // TODO: checksum!
    match deflate_decompress(bit_reader, output) {
      Some(deflate_err) => Some(~DeflatingError(deflate_err)),
      None => None
    }
  }
}

#[cfg(test)]
mod test {
  use zlib::decompress::{decompress};
  use zlib::error::*;
  use deflate::error::*;

  fn decompress_(bytes: &[u8]) -> Result<~[u8], ~ZlibError> {
    let mut err = None;
    let bytes = do io::with_bytes_writer |writer| {
      do io::with_bytes_reader(bytes) |reader| {
        err = decompress(reader, writer);
      }
    };

    match err {
      Some(zlib_err) => Err(zlib_err),
      None => Ok(bytes)
    }
  }

  #[test]
  fn test_decompress() {
    let bytes = &[120,156,99,98,102,101,231,230,5,0,0,109,0,42];
    assert_eq!(decompress_(bytes).unwrap(), ~[2,3,5,7,11,13]);
  }

  #[test]
  fn test_decompress_errors() {
    /* unknown compression method */
    match decompress_(&[0b0000_0101, 0]) {
      Err(~UnknownCompressionMethod(5)) => { },
      Err(err) => fail!(fmt!("unexpected %s", err.to_str())),
      Ok(_) => fail!(~"expected an error")
    }

    /* FCHECK doesn't match */
    match decompress_(&[0b0111_1000, 0b10_0_11101]) {
      Err(~FlagsCorrupted) => { },
      Err(err) => fail!(fmt!("unexpected %s", err.to_str())),
      Ok(_) => fail!(~"expected an error")
    }
    
    /* FDICT is on */
    match decompress_(&[0b0111_1000, 0b10_1_00110]) {
      Err(~PresetDictionaryUsed) => { },
      Err(err) => fail!(fmt!("unexpected %s", err.to_str())),
      Ok(_) => fail!(~"expected an error")
    }
  }

  #[test]
  fn test_decompress_deflate_errors() {
    /* bad deflate block */
    match decompress_(&[120,156,0b110,0,0,0,0,0]) {
      Err(~DeflatingError(~BadBlockType)) => { },
      Err(err) => fail!(fmt!("unexpected %s", err.to_str())),
      Ok(_) => fail!(~"expected an error")
    }
  }
}
