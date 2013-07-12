use gzip::header;
use gzip::error;

pub struct HeaderDecoder {
  priv x: (),
}

impl HeaderDecoder {
  pub fn new() -> HeaderDecoder {
    fail!()
  }

  pub fn input<'a>(self, _chunk: &'a [u8]) 
    -> Either<HeaderDecoder, (Result<~header::Header, ~error::Error>, &'a [u8])>
  {
    fail!()
  }
}

#[cfg(test)]
mod test {
  use gzip::test_helpers::*;
  use gzip::header;
  use gzip::error;

  fn header(f: &once fn(&mut header::Header)) -> ~header::Header {
    let mut header = ~header::Header::blank();
    f(header);
    header
  }

  #[test]
  fn test_decode_header_ok() {
    { // blank header
      assert_eq!(decode_hdr_ok(&[
          0x1f, 0x8b, // magic
          8, 0b000_00000, // cm, flags
          0, 0, 0, 0, // mtime
          0, 255, // extra flags, system
          // no compressed data
        ]),
        ~header::Header::blank()
      );
    }

    { // set mtime, system and extra flags

      assert_eq!(decode_hdr_ok(&[
          0x1f, 0x8b, 0x08, 0x00,
          0x21, 0x43, 0x65, 0x87,
          0x00, 0x01,
          0xe3, 0x12, 0x91, 0x03, 0x00a,
          0xf2, 0xb6, 0x77, 0x26,
          0x03, 0x00, 0x00, 0x00
        ]), do header |h| {
          h.extra_flags = 42;
          h.mtime = Some(0x87654321);
          h.system = Some(header::Amiga);
        }
      );
    }
  }

  #[test]
  fn test_decode_header_err() {
    { // bad magic number
      assert_eq!(decode_hdr_err(&[
          0x1f, 0x8a, 2, 3, 5, 7
        ]),
        (~error::BadMagicNumber(0x8b_1f, 0x8a_1f), &[2, 3, 5, 7])
      );
    }

    { // bad compression method
      assert_eq!(decode_hdr_err(&[
          0x1f, 0x8b, 77, 2, 3, 5, 7
        ]),
        (~error::BadCompressionMethod(77), &[2, 3, 5, 7])
      );
    }

    { // reserved flag set on
      assert_eq!(decode_hdr_err(&[
          0x1f, 0x8b, 8, 0b010_00000, 2, 3, 5, 7
        ]),
        (~error::ReservedFlagUsed(6), &[2, 3, 5, 7])
      );
    }
  }

  #[test]
  fn test_decode_header_extras() {
    { // no extra field
      assert_eq!(decode_hdr_ok(&[
          0x1f, 0x8b, 8, 0b000_00100,
          0, 0, 0, 0, 0, 255, 
          0, 0, 
          0, 0, 0, 0, 0, 0, 0, 0, 
        ]),
        do header |h| {
          h.extras = Some(~[]);
        }
      );
    }

    { // two small extra fields
      assert_eq!(decode_hdr_ok(&[
          0x1f, 0x8b, 8, 0b000_00100,
          0, 0, 0, 0, 0, 255, 
          14, 0, 
            11, 22, 3, 110, 120, 130,
            44, 2, 5, 2, 3, 5, 7, 11,
          0, 0, 0, 0, 0, 0, 0, 0, 
        ]), do header |h| {
          h.extras = Some(~[
              header::Extra {
                id: (11, 22),
                data: ~[110, 120, 130],
              },
              header::Extra {
                id: (44, 2),
                data: ~[2, 3, 5, 7, 11],
              }
            ]);
        }
      );
    }
  }

  #[test]
  fn test_decode_header_file_name() {
    assert_eq!(decode_hdr_ok(&[
        0x1f, 0x8b, 8, 0b000_01000,
        0, 0, 0, 0, 0, 255, 
          100, 101, 99, 111, 100, 101, 114, 46, 114, 115, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 
      ]),
      do header |h| {
        h.file_name = Some(~"decoder.rs");
      }
    );
  }

  #[test]
  fn test_decode_header_comment() {
    assert_eq!(decode_hdr_ok(&[
        0x1f, 0x8b, 8, 0b000_10000,
        0, 0, 0, 0, 0, 255, 
          67, 114, 101, 97, 116, 101, 100, 32, 98,
          121, 32, 99, 111, 109, 112, 114, 115, 114, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 
      ]),
      do header |h| {
        h.file_name = Some(~"Created by comprsr");
      }
    );
  }

  #[test]
  fn test_decode_header_crc() {
    { // CRC is ok
      assert_eq!(decode_hdr_ok(&[
          0x1f, 0x8b, 8, 0b000_10110,
          0xef, 0xbe, 0xad, 0xde,
          0, 255,
          7, 0,   2, 3, 1, 2, 3, 5, 8,
          70, 105, 98, 111, 110, 97, 99, 99, 105, 0,
          0xc0, 0x71,
          0, 0, 0, 0, 0, 0, 0, 0
        ]),
        do header |h| {
          h.mtime = Some(0xdead_beef);
          h.extras = Some(~[
              header::Extra { id: (2, 3), data: ~[1, 2, 3, 5, 8], }
            ]);
          h.comment = Some(~"Fibonacci");
          h.has_crc = true;
        }
      );
    }

    { // CRC is wrong
      assert_eq!(decode_hdr_err(&[
          0x1f, 0x8b, 8, 0b000_10110,
          0xef, 0xbe, 0xad, 0xde,
          0, 255,
          7, 0,   2, 3, 1, 2, 3, 5, 8,
          70, 105, 98, 111, 110, 97, 99, 99, 105, 0,
          0xc0, 0xdd,
          2, 3, 4, 5,
        ]),
        (~error::BadHeaderChecksum(0x71c0, 0xddc0), &[2, 3, 4, 5])
      );
    }
  }
}
