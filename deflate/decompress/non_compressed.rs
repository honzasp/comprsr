use deflate::bit_reader::{BitReader};
use deflate::output::{Output};
use deflate::error::*;

pub fn non_compressed_block(in: &mut BitReader, out: &mut Output)
-> Option<~DeflateError> 
{
  let lsb = in.read_byte();
  let msb = in.read_byte();
  let nlsb = in.read_byte();
  let nmsb = in.read_byte();

  let len: u16 = lsb as u16 | (msb as u16 << 8);
  let nlen: u16 = nlsb as u16 | (nmsb as u16 << 8);

  if len == !nlen {
    out.copy_bytes(len as uint, in)
  } else {
    Some(~LengthMismatchError(len as u16, nlen as u16))
  }
}

