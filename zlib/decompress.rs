use zlib::error::*;
use deflate_decompress = deflate::decompress::decompress;
use deflate::bit_reader::{BitReader};
use deflate::output::{Output};
use zlib::adler32_writer::{Adler32Writer};

pub fn decompress(in: @io::Reader, out: @io::Writer) 
  -> Option<~ZlibError> 
{
  let cmf = in.read_byte();
  let flg = in.read_byte();

  if cmf < 0 || flg < 0 {
    return Some(~MissingHeaderError);
  }

  let cm = (cmf & 0b1111) as uint;
  let fdict = (flg & 0b100000) != 0;

  let a32_out = Adler32Writer::new(out);

  if cm != 8 {
    Some(~UnknownCompressionMethodError(cm))
  } else if fdict {
    Some(~PresetDictionaryUsedError)
  } else if (cmf as uint * 256 + flg as uint) % 31 != 0 {
    Some(~FlagsCorruptedError)
  } else {
    let mut bit_reader = BitReader::new(in);
    let mut output = Output::new(a32_out as @io::Writer);

    match deflate_decompress(bit_reader, output) {
      Some(deflate_err) => Some(~DeflatingError(deflate_err)),
      None => {
        let expected = a32_out.adler32();
        let mut buf = ~[0,0,0,0];

        if in.read(buf, 4) != 4 {
          Some(~MissingChecksumError(expected))
        } else {
          let got =
            (buf[0] as u32 << 24) |
            (buf[1] as u32 << 16) |
            (buf[2] as u32 << 8) |
            buf[3] as u32;

          if expected != got {
            Some(~ChecksumMismatchError(expected, got))
          } else {
            None
          }
        }
      }
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
    /* missing header */
    match decompress_(&[120]) {
      Err(~MissingHeaderError) => { },
      Err(err) => fail!(fmt!("unexpected %s", err.to_str())),
      Ok(_) => fail!(~"expected an error")
    }

    /* unknown compression method */
    match decompress_(&[0b0000_0101, 0]) {
      Err(~UnknownCompressionMethodError(5)) => { },
      Err(err) => fail!(fmt!("unexpected %s", err.to_str())),
      Ok(_) => fail!(~"expected an error")
    }

    /* FCHECK doesn't match */
    match decompress_(&[0b0111_1000, 0b10_0_11101]) {
      Err(~FlagsCorruptedError) => { },
      Err(err) => fail!(fmt!("unexpected %s", err.to_str())),
      Ok(_) => fail!(~"expected an error")
    }
    
    /* FDICT is on */
    match decompress_(&[0b0111_1000, 0b10_1_00110]) {
      Err(~PresetDictionaryUsedError) => { },
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

  #[test]
  fn test_decompress_checksum_errors() {
    /* bad checksum */
    let bytes1 = &[120,156,99,98,102,101,231,230,5,0,0,109,1,42];
    match decompress_(bytes1) {
      Err(~ChecksumMismatchError(7_143_466, 7_143_722)) => { },
      Err(err) => fail!(fmt!("unexpected %s", err.to_str())),
      Ok(_) => fail!(~"expected an error")
    }

    /* missing part of checksum */
    let bytes2 = &[120,156,99,98,102,101,231,230,5,0,0,109];
    match decompress_(bytes2) {
      Err(~MissingChecksumError(109*256*256+42)) => { },
      Err(err) => fail!(fmt!("unexpected %s", err.to_str())),
      Ok(_) => fail!(~"expected an error")
    }
  }
}
