use inflate::error;
use inflate::inflater;

pub struct BitReader<'self> {
  priv rest: &'self [u8],
}

pub struct BitBuf {
  priv x: uint,
}

impl<'self> BitReader<'self> {

  pub fn with_buf<'a>(
    bit_buf: &BitBuf,
    chunk: &'a [u8],
    body: &fn(&mut BitReader) -> Option<Result<(),~error::Error>>
  ) -> inflater::Res<&'a [u8]>
  {
    let mut bit_reader = BitReader { rest: chunk };
    match body(&mut bit_reader) {
      None => {
        // save the possible remaining byte to bit_buf
        inflater::ConsumedRes
      },
      Some(res) => {
        let rest = bit_reader.rest;
        match res {
          Ok(())   => inflater::FinishedRes(rest),
          Err(err) => inflater::ErrorRes(err, rest),
        }
      }
    }
  }

  pub fn has_bits(&self, bits: uint) -> bool {
    fail!()
  }

  pub fn has_bytes(&self, bytes: uint) -> bool {
    fail!()
  }

  pub fn skip_to_byte(&mut self) {
    fail!()
  }

  pub fn read_bits8(&mut self, bits: uint) -> u8 {
    fail!()
  }

  pub fn read_bits16(&mut self, bits: uint) -> u16 {
    fail!()
  }

  pub fn read_u16(&mut self) -> u16 {
    fail!()
  }

  pub fn read_byte_chunk(&mut self, limit: uint) -> &'self [u8] {
    fail!()
  }

  pub fn rest_bit_buf(self) -> BitBuf {
    fail!()
  }
}

impl BitBuf {
  pub fn new() -> BitBuf {
    fail!()
  }
}
