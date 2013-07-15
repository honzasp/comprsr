use bits;
use gzip::header;
use gzip::error;
use std::vec;
use std::uint;
use std::str; 

pub struct HeaderDecoder {
  priv stage: Stage,
  priv byte_buf: bits::ByteBuf,
  priv header: ~header::Header,
}

enum Stage {
  BeginStage(),
  ExtraStage(),
  ExtraHeaderStage(uint),
  ExtraDataStage((u8, u8), ~[u8], uint, uint),
  FileNameStage(),
  FileNameDataStage(~[u8]),
  CommentStage(),
  CommentDataStage(~[u8]),
  CrcStage(),
  ErrorStage(~error::Error),
  EndStage(),
}

impl HeaderDecoder {
  pub fn new() -> HeaderDecoder {
    HeaderDecoder {
      stage: BeginStage,
      byte_buf: bits::ByteBuf::new(),
      header: ~header::Header::blank(),
    }
  }

  pub fn input<'a>(self, chunk: &'a [u8]) 
    -> Either<HeaderDecoder, (Result<~header::Header, ~error::Error>, &'a [u8])>
  {
    let HeaderDecoder { stage, byte_buf, header } = self;
    let mut m_stage = stage;
    let m_byte_buf = byte_buf;
    let mut m_header = header;

    let mut reader = bits::ByteReader::new(m_byte_buf, chunk);

    loop {
      let (continue, new_stage) = match m_stage {
        BeginStage => {
          if reader.has_bytes(10) {
            let id    = reader.read_u16_le();
            let cm    = reader.read_byte();
            let flg   = reader.read_byte();
            let mtime = reader.read_u32_le();
            let xfl   = reader.read_byte();
            let os    = reader.read_byte();

            let ftext     = flg & 0b1;
            let fhcrc     = flg & 0b10;
            let fextra    = flg & 0b100;
            let fname     = flg & 0b1000;
            let fcomment  = flg & 0b1_0000;
            let reserved5 = flg & 0b10_0000;
            let reserved6 = flg & 0b100_0000;
            let reserved7 = flg & 0b1000_0000;

            if id != 0x8b_1f {
              (true, ErrorStage(~error::BadMagicNumber(0x8b_1f, id)))
            } else if cm != 8 {
              (true, ErrorStage(~error::BadCompressionMethod(cm as uint)))
            } else if reserved5 != 0 {
              (true, ErrorStage(~error::ReservedFlagUsed(5)))
            } else if reserved6 != 0 {
              (true, ErrorStage(~error::ReservedFlagUsed(6)))
            } else if reserved7 != 0 {
              (true, ErrorStage(~error::ReservedFlagUsed(7)))
            } else {
              m_header.is_text     = ftext != 0;
              m_header.has_crc     = fhcrc != 0;
              m_header.extras      = if fextra != 0 { Some(~[]) } else { None };
              m_header.file_name   = if fname != 0 { Some(~"") } else { None };
              m_header.comment     = if fcomment != 0 { Some(~"") } else { None };
              m_header.extra_flags = xfl;
              m_header.system      = header::System::from_number(os);
              m_header.mtime       = if mtime != 0 { Some(mtime) } else { None };
              (true, ExtraStage)
            }
          } else {
            (false, BeginStage)
          }
        },
        ExtraStage => {
          if m_header.extras.is_some() {
            if reader.has_bytes(2) {
              let xlen = reader.read_u16_le();
              (true, ExtraHeaderStage(xlen as uint))
            } else {
              (false, ExtraStage)
            }
          } else {
            (true, FileNameStage)
          }
        },
        ExtraHeaderStage(xlen_rem) => {
          if xlen_rem == 0 {
            (true, FileNameStage)
          } else if xlen_rem < 4 {
            (true, ErrorStage(~error::TrailingExtraBytes(xlen_rem)))
          } else if reader.has_bytes(4) {
            let si1 = reader.read_byte();
            let si2 = reader.read_byte();
            let len = reader.read_u16_le();

            if len as uint <= xlen_rem - 4 {
              (true, ExtraDataStage((si1, si2), ~[],
                len as uint, xlen_rem - 4))
            } else {
              (true, ErrorStage(~error::ExtraTooLong(xlen_rem - 4, len as uint)))
            }
          } else {
            (false, ExtraHeaderStage(xlen_rem))
          }
        },
        ExtraDataStage(id, data, len_rem, xlen_rem) => {
          if len_rem == 0 {
            let extra = header::Extra { id: id, data: data };
            m_header.extras.get_mut_ref().push(extra);
            (true, ExtraHeaderStage(xlen_rem))
          } else if reader.has_some_bytes() {
            do reader.consume_chunk(data) |data, whole_chunk| {
              let (chunk, opt_rest) =
                if whole_chunk.len() <= len_rem {
                  (whole_chunk, None)
                } else {
                  ( whole_chunk.slice(0, len_rem)
                  , Some(whole_chunk.slice(len_rem, whole_chunk.len())))
                };

              ((true, ExtraDataStage(id, vec::append(data, chunk),
                len_rem - chunk.len(), xlen_rem - chunk.len())), opt_rest)
            }
          } else {
            (false, ExtraDataStage(id, data, len_rem, xlen_rem))
          }
        },
        FileNameStage => {
          if m_header.file_name.is_some() {
            (true, FileNameDataStage(~[]))
          } else {
            (true, CommentStage)
          }
        },
        FileNameDataStage(read_yet) => {
          if reader.has_some_bytes() {
            do reader.consume_chunk(read_yet) |read_yet, whole_chunk| {
              let mut data = read_yet;
              let mut opt_rest = None;

              for uint::range(0, whole_chunk.len()) |i| {
                if whole_chunk[i] == 0 {
                  opt_rest = Some(whole_chunk.slice(i + 1, whole_chunk.len()));
                  break
                } else {
                  data.push(whole_chunk[i]);
                }
              }

              match opt_rest {
                None => ((true, FileNameDataStage(data)), None),
                Some(rest) => {
                  m_header.file_name = Some(str::from_bytes_owned(data));
                  ((true, CommentStage), Some(rest))
                },
              }
            }
          } else {
            (false, FileNameDataStage(read_yet))
          }
        },
        CommentStage => {
          if m_header.comment.is_some() {
            (true, CommentDataStage(~[]))
          } else {
            (true, CrcStage)
          }
        },
        // TODO: dry! (a copy of FileNameDataStage!)
        CommentDataStage(read_yet) => {
          if reader.has_some_bytes() {
            do reader.consume_chunk(read_yet) |read_yet, whole_chunk| {
              let mut data = read_yet;
              let mut opt_rest = None;

              for uint::range(0, whole_chunk.len()) |i| {
                if whole_chunk[i] == 0 {
                  opt_rest = Some(whole_chunk.slice(i + 1, whole_chunk.len()));
                  break
                } else {
                  data.push(whole_chunk[i]);
                }
              }

              match opt_rest {
                None => ((true, CommentDataStage(data)), None),
                Some(rest) => {
                  m_header.comment = Some(str::from_bytes_owned(data));
                  ((true, CrcStage), Some(rest))
                },
              }
            }
          } else {
            (false, CommentDataStage(read_yet))
          }
        },
        CrcStage => {
          if m_header.has_crc {
            if reader.has_bytes(2) {
              let read_crc  = reader.read_u16_le();
              let actual_crc = (m_header.crc32().crc32() & 0xff_ff) as u16;
              if read_crc == actual_crc {
                (true, EndStage)
              } else {
                let err = ~error::BadHeaderChecksum(actual_crc, read_crc);
                (true, ErrorStage(err))
              }
            } else {
              (false, CrcStage)
            }
          } else {
            (true, EndStage)
          }
        },
        ErrorStage(err) => {
          // TODO: create a method reader.close_drop_buf and update other decoders to use it
          let (_byte_buf, rest) = reader.close();
          return Right((Err(err), rest))
        },
        EndStage => {
          let (_byte_buf, rest) = reader.close();
          return Right((Ok(m_header), rest))
        },
      };

      if !continue {
        let decoder = HeaderDecoder {
          stage: new_stage,
          byte_buf: reader.close_to_buf(),
          header: m_header
        };
        return Left(decoder)
      } else {
        m_stage = new_stage;
      }
    }
  }
}

#[cfg(test)]
mod test {
  use gzip::test_helpers::*;
  use gzip::header;
  use gzip::error;
  use std::uint;

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
          0xab, 0x01,
        ]), do header |h| {
          h.extra_flags = 0xab;
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
          0x1f, 0x8a, 3,4,5,6,7,8,9,10,11
        ]),
        (~error::BadMagicNumber(0x8b_1f, 0x8a_1f), &[11])
      );
    }

    { // bad compression method
      assert_eq!(decode_hdr_err(&[
          0x1f, 0x8b, 77, 4,5,6,7,8,9,10,11
        ]),
        (~error::BadCompressionMethod(77), &[11])
      );
    }

    { // reserved flag set on
      assert_eq!(decode_hdr_err(&[
          0x1f, 0x8b, 8, 0b010_00000, 5,6,7,8,9,10,11
        ]),
        (~error::ReservedFlagUsed(6), &[11])
      );
    }
  }

  #[test]
  fn test_decode_header_extras() {
    { // empty extra field
      assert_eq!(decode_hdr_ok(&[
          0x1f, 0x8b, 8, 0b000_00100,
          0, 0, 0, 0, 0, 255, 
          0, 0, 
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
          16, 0, 
            11, 22, 3, 0, 110, 120, 130,
            44, 2,  5, 0, 2, 3, 5, 7, 11,
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
  fn test_decode_header_extras_err() {
    { // trailing bytes (there are 3 remaining bytes according to xlen, but they
      // cannot form a subfield header of length 4
      assert_eq!(decode_hdr_err(&[
          0x1f, 0x8b, 8, 0b000_00100,
          0, 0, 0, 0, 0, 255, 
          9, 0,
            44, 55, 2, 0, 10, 20,
          1, 2, 3, 4, 5
        ]), (~error::TrailingExtraBytes(3), &[1, 2, 3, 4, 5])
      );
    }

    { // subfield too long
      assert_eq!(decode_hdr_err(&[
          0x1f, 0x8b, 8, 0b000_00100,
          0, 0, 0, 0, 0, 255, 
          15, 0,
            44, 55, 2, 0, 10, 20,
            66, 77, 50, 0,
          2, 3, 5, 7, 11
        ]), (~error::ExtraTooLong(5, 50), &[2, 3, 5, 7, 11])
      );
    }
  }

  #[test]
  fn test_decode_header_file_name() {
    assert_eq!(decode_hdr_ok(&[
        0x1f, 0x8b, 8, 0b000_01000,
        0, 0, 0, 0, 0, 255, 
          100, 101, 99, 111, 100, 101, 114, 46, 114, 115, 0,
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
      ]),
      do header |h| {
        h.comment = Some(~"Created by comprsr");
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
          9, 0,
          2, 3,  5, 0,   1, 2, 3, 5, 8,
          70, 105, 98, 111, 110, 97, 99, 99, 105, 0,
          0x0e, 0x1a,
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
          9, 0,
          2, 3,  5, 0,   1, 2, 3, 5, 8,
          70, 105, 98, 111, 110, 97, 99, 99, 105, 0,
          0xad, 0xde,
          2, 3, 4, 5,
        ]),
        (~error::BadHeaderChecksum(0x1a0e, 0xdead), &[2, 3, 4, 5])
      );
    }
  }

  #[test]
  fn test_decode_header_chunked() {
    for uint::range(1, 10) |chunk_len| {
      assert_eq!(decode_hdr_chunked_ok(chunk_len, &[
          0x1f, 0x8b, 8, 0b000_00100,
          0, 0, 0, 0, 0, 255, 
          16, 0, 
            11, 22, 3, 0, 110, 120, 130,
            44, 2,  5, 0, 2, 3, 5, 7, 11,
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

      assert_eq!(decode_hdr_chunked_ok(chunk_len, &[
          0x1f, 0x8b, 0x08, 0x00,
          0x21, 0x43, 0x65, 0x87,
          0xab, 0x01,
        ]), do header |h| {
          h.extra_flags = 0xab;
          h.mtime = Some(0x87654321);
          h.system = Some(header::Amiga);
        }
      );

      assert_eq!(decode_hdr_chunked_ok(chunk_len, &[
          0x1f, 0x8b, 8, 0b000_10000,
          0, 0, 0, 0, 0, 255, 
            67, 114, 101, 97, 116, 101, 100, 32, 98,
            121, 32, 99, 111, 109, 112, 114, 115, 114, 0,
        ]),
        do header |h| {
          h.comment = Some(~"Created by comprsr");
        }
      );
    }
  }
}
