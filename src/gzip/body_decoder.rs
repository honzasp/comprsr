use bits::recv;
use gzip::error;

pub struct BodyDecoder {
  priv x: (),
}

impl BodyDecoder {
  pub fn new() -> BodyDecoder {
    fail!()
  }

  pub fn input<'a, R: recv::Recv<u8>>(self, chunk: &'a [u8], recv: R)
    -> (Either<BodyDecoder, (Result<(), ~error::Error>, &'a [u8])>, R)
  {
    fail!()
  }
}

#[cfg(test)]
mod test {
  use gzip::test_helpers::*;
  use gzip::header;
  use gzip::error;

  #[test]
  fn test_decode_body_ok() {
    assert_eq!(decode_body_ok(&[
        0x1f, 0x8b, 8, 0b000_00000,
        0, 0, 0, 0, 0, 255, 
        0x63, 0x64, 0x64, 0x62, 0x66, 0xe5,
        0xe0, 0x15, 0x55, 0x32, 0x07, 0x00,
        0xf0, 0x8a, 0xcb, 0xff,
        0x0a, 0x00, 0x00, 0x00,
      ]),
      ~[1, 1, 2, 3, 5, 8, 13, 21, 34, 55]);
  }

  #[test]
  fn test_decode_body_err() {
    { // bad data checksum
      assert_eq!(decode_body_err(&[
          0x1f, 0x8b, 8, 0b000_00000,
          0, 0, 0, 0, 0, 255, 
          0x63, 0x64, 0x64, 0x62, 0x66, 0xe5,
          0xe0, 0x15, 0x55, 0x32, 0x07, 0x00,
          0xef, 0xbe, 0xad, 0xde,
          2, 3, 4, 5,
        ]),
        (~error::BadDataChecksum(0xffcb8af0, 0xdeadbeef), &[2, 3, 4, 5])
      );
    }

    { // bad input data size
      assert_eq!(decode_body_err(&[
          0x1f, 0x8b, 8, 0b000_00000,
          0, 0, 0, 0, 0, 255, 
          0x63, 0x64, 0x64, 0x62, 0x66, 0xe5,
          0xe0, 0x15, 0x55, 0x32, 0x07, 0x00,
          0xf0, 0x8a, 0xcb, 0xff,
          77, 0, 0, 0,
          2, 3, 4, 5,
        ]),
        (~error::BadDataSize(10, 77), &[2, 3, 4, 5])
      );
    }
  }
}
