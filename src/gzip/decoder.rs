use gzip::error;
use gzip::member_recv;

pub struct Decoder<R> {
  priv x: (),
}

// TODO: InputRes and FinishRes are isomorphic to Option

#[deriving(Eq)]
pub enum InputRes<A> {
  pub ConsumedRes(),
  pub ErrorRes(~error::Error, A),
}

#[deriving(Eq)]
pub enum FinishRes {
  pub FinishedRes(),
  pub ErrorFinRes(~error::Error),
}

impl<R: member_recv::MemberReceiver<S>, S> Decoder<R> {
  pub fn new(_member_recv: ~R) -> Decoder<R> {
    fail!();
  }

  pub fn close(self) -> ~R {
    fail!();
  }

  pub fn input<'a>(&mut self, _chunk: &'a [u8]) -> InputRes<&'a [u8]> {
    fail!();
  }

  pub fn finish(&mut self) -> FinishRes {
    fail!();
  }

  pub fn has_finished(&self) -> bool {
    fail!();
  }

  pub fn get_error(&self) -> Option<~error::Error> {
    fail!();
  }

  pub fn is_error(&self) -> bool {
    fail!();
  }

  pub fn is_ready(&self) -> bool {
    !self.has_finished()
  }
}

#[cfg(test)]
mod test {
  use gzip::test_helpers::*;
  use gzip::header;
  use gzip::error;

  #[test]
  fn test_decode_ok() {
    { // blank header, small body
      assert_eq!(decode_ok1(&[
          0x1f, 0x8b, 8, 0b000_00000,
          0, 0, 0, 0, 0, 255, 
          0x63, 0x64, 0x64, 0x62, 0x66, 0xe5,
          0xe0, 0x15, 0x55, 0x32, 0x07, 0x00,
          0xf0, 0x8a, 0xcb, 0xff,
          0x0a, 0x00, 0x00, 0x00,
        ]),
        (~header::Header::blank(), ~[1, 1, 2, 3, 5, 8, 13, 21, 34, 55])
      );
    }
  }

  #[test]
  fn test_decode_err() {
    { // bad data checksum
      assert_eq!(decode_err(&[
          0x1f, 0x8b, 8, 0b000_00000,
          0, 0, 0, 0, 0, 255, 
          0x63, 0x64, 0x64, 0x62, 0x66, 0xe5,
          0xe0, 0x15, 0x55, 0x32, 0x07, 0x00,
          0xef, 0xbe, 0xad, 0xde,
          2, 3, 4, 5,
        ]),
        (~error::BadDataChecksum(0xffcb8af0, 0xdeadbeef), Some(&[2, 3, 4, 5]))
      );
    }

    { // bad input data size
      assert_eq!(decode_err(&[
          0x1f, 0x8b, 8, 0b000_00000,
          0, 0, 0, 0, 0, 255, 
          0x63, 0x64, 0x64, 0x62, 0x66, 0xe5,
          0xe0, 0x15, 0x55, 0x32, 0x07, 0x00,
          0xf0, 0x8a, 0xcb, 0xff,
          77, 0, 0, 0,
          2, 3, 4, 5,
        ]),
        (~error::BadDataSize(10, 77), Some(&[2, 3, 4, 5]))
      );
    }

    { // unterminated data
      assert_eq!(decode_err(&[
          0x1f, 0x8b, 8, 0b000_00000,
          0, 0, 0, 0, 0, 255, 
          0x63, 0x64, 0x64, 0x62, 0x66, 0xe5,
        ]),
        (~error::UnterminatedData(6), None)
      );
    }

    { // unterminated checksum
      assert_eq!(decode_err(&[
          0x1f, 0x8b, 8, 0b000_00000,
          0, 0, 0, 0, 0, 255, 
          0x63, 0x64, 0x64, 0x62, 0x66, 0xe5,
          0xe0, 0x15, 0x55, 0x32, 0x07, 0x00,
          0xf0, 0x8a, 0xcb,
        ]),
        (~error::UnterminatedDataChecksum(4, 3), None)
      );
    }

    { // unterminated data size
      assert_eq!(decode_err(&[
          0x1f, 0x8b, 8, 0b000_00000,
          0, 0, 0, 0, 0, 255, 
          0x63, 0x64, 0x64, 0x62, 0x66, 0xe5,
          0xe0, 0x15, 0x55, 0x32, 0x07, 0x00,
          0xf0, 0x8a, 0xcb, 0xff,
          0x0a, 0x00, 
        ]),
        (~error::UnterminatedDataSize(4, 2), None)
      );
    }
  }

  #[test]
  fn test_decode_many_members() {
    { // no members
      assert_eq!(decode_ok(&[]), ~[]);
    }

    { // two blank members
      assert_eq!(decode_ok(&[
          0x1f, 0x8b, 8, 0b000_00000,
          0, 0, 0, 0, 0, 255, 
          0, 0, 0, 0, 0, 0, 0, 0, 

          0x1f, 0x8b, 8, 0b000_00000,
          0, 0, 0, 0, 0, 255, 
          0, 0, 0, 0, 0, 0, 0, 0, 
        ]),
        ~[
          (~header::Header::blank(), ~[]),
          (~header::Header::blank(), ~[]),
        ]
      );
    }

    { // two small members
      assert_eq!(decode_ok(&[
          0x1f, 0x8b, 8, 0b000_00000,
          0, 0, 0, 0, 0, 255,
          0x63, 0x64, 0x64, 0x62, 0x66, 0x05, 0x00,
          0xea, 0xca, 0x3d, 0x1b,
          0x05, 0x00, 0x00, 0x00,

          0x1f, 0x8b, 8, 0b000_00000,
          0, 0, 0, 0, 0, 255,
          0xe3, 0xe0, 0x15, 0x55, 0x32, 0x07, 0x00,
          0xb5, 0xe6, 0xda, 0x01,
          0x05, 0x00, 0x00, 0x00,
        ]),
        ~[
          (~header::Header::blank(), ~[1, 1, 2, 3, 5]),
          (~header::Header::blank(), ~[8, 13, 21, 34, 55]),
        ]);
    }
  }
}
