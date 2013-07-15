use bits;
use inflate::inflater;
use checksums::adler32;
use zlib::error;

struct Decoder {
  priv stage: Stage,
  priv byte_buf: bits::ByteBuf,
}

enum Stage {
  HeaderStage,
  DataStage(inflater::Inflater, adler32::Adler32),
  Adler32Stage(u32),
  ErrorStage(~error::Error),
  EndStage,
}

impl Decoder {
  pub fn new() -> Decoder {
    Decoder { 
      stage: HeaderStage,
      byte_buf: bits::ByteBuf::new(),
    }
  }

  pub fn input<'a, R: bits::recv::Recv<u8>>
    (self, chunk: &'a [u8], recv: R) 
    -> (Either<Decoder, (Result<(), ~error::Error>, &'a [u8])>, R)
  {
    let Decoder { stage, byte_buf } = self;
    let i_byte_buf = byte_buf;
    let mut i_stage = stage;

    let mut recv = recv;
    let mut byte_reader = bits::ByteReader::new(i_byte_buf, chunk);

    loop {
      let (continue, new_stage) = match i_stage {
        HeaderStage => {
          if byte_reader.has_bytes(2) {
            let cmf = byte_reader.read_byte();
            let flg = byte_reader.read_byte();

            let cm = cmf & 0b1111;
            let cinfo = (cmf >> 4) & 0b1111;

            let _fcheck = flg & 0b11111;
            let fdict = (flg >> 5) & 0b1;
            let _flevel = (flg >> 6) & 0b11;

            let win_size: uint = 1 << (8 + cinfo as uint);

            if cm != 8 {
              (true, ErrorStage(~error::BadCompressionMethod(cm as uint)))
            } else if win_size > 32 * 1024 {
              (true, ErrorStage(~error::WindowTooLong(win_size)))
            } else if (cmf as uint * 256 + flg as uint) % 31 != 0 {
              (true, ErrorStage(~error::BadHeaderChecksum(cmf, flg)))
            } else if fdict != 0 {
              (true, ErrorStage(~error::DictionaryUsed))
            } else {
              (true, DataStage(inflater::Inflater::new(), adler32::Adler32::new()))
            }
          } else {
            (false, HeaderStage)
          }
        },
        DataStage(inflater, a32) => {
          let a32 = a32;
          let inflater = inflater;

          if byte_reader.has_some_bytes() {
            // TODO: somehow get rid of the extra argument to `consume_chunk` 
            // (Rust doesn't allow us to move from the captured variables in the closure, because
            // the once-fn doesn't *have* to be called, so the value may or may not be moved, which
            // is unsound.)
            let (new_stage, new_recv) = do byte_reader.consume_chunk((inflater, a32, recv)) 
              |(inflater, a32, recv), chunk| {

              let (res, (new_recv, new_a32)) = inflater.input(chunk, (recv, a32));

              match res {
                Left(new_inflater) => 
                  ((DataStage(new_inflater, new_a32), new_recv), None),
                Right((Ok(()), rest)) =>
                  ((Adler32Stage(new_a32.adler32()), new_recv), Some(rest)),
                Right((Err(err), rest)) =>
                  ((ErrorStage(~error::InflateError(err)), new_recv), Some(rest)),
              }
            };

            recv = new_recv;
            (true, new_stage)
          } else {
            (false, DataStage(inflater, a32))
          }
        },
        Adler32Stage(computed_checksum) => {
          if byte_reader.has_bytes(4) {
            let read_checksum = byte_reader.read_u32_be();
            if computed_checksum == read_checksum {
              (true, EndStage)
            } else {
              (true, ErrorStage(~error::BadDataChecksum
                (computed_checksum, read_checksum)))
            }
          } else {
            (false, Adler32Stage(computed_checksum))
          }
        },
        EndStage => {
          let (_byte_buf, rest) = byte_reader.close();
          return (Right((Ok(()), rest)), recv)
        },
        ErrorStage(err) => {
          let (_byte_buf, rest) = byte_reader.close();
          return (Right((Err(err), rest)), recv)
        },
      };

      if continue {
        i_stage = new_stage;
      } else {
        let decoder = Decoder { stage: new_stage, byte_buf: byte_reader.close_to_buf() };
        return (Left(decoder), recv)
      }
    }
  }
}

#[cfg(test)]
mod test {
  use std::uint;
  use zlib::decoder;
  use zlib::error;
  use inflate;

  fn decode_ok(bytes: &[u8]) -> ~[u8] {
    let decoder = decoder::Decoder::new();

    match decoder.input(bytes, ~[]) {
      (Right((Ok(()), [])), buf) => buf,
      x => fail!(fmt!("decode_ok: unexpected %?", x)),
    }
  }

  fn decode_err<'a>(bytes: &'a [u8]) -> (~error::Error, &'a [u8]) {
    let decoder = decoder::Decoder::new();

    match decoder.input(bytes, ()) {
      (Right((Err(err), rest)), ()) => (err, rest),
      x => fail!(fmt!("decode_err: unexpected %?", x)),
    }
  }

  fn decode_chunked_ok(chunk_len: uint, bytes: &[u8]) -> ~[u8] {
    let mut decoder = decoder::Decoder::new();
    let mut out: ~[u8] = ~[];

    let mut iter = bytes.chunk_iter(chunk_len);
    loop {
      match iter.next() {
        Some(chunk) => {
          let (result, new_out) = decoder.input(chunk, out);
          out = new_out;
          match result {
            Left(new_decoder) => { decoder = new_decoder },
            Right((Ok(()), [])) => { return out },
            x => fail!(fmt!("decode_chunked_ok: unexpected %?", x)),
          }
        },
        None => fail!("decode_chunked_ok: decoder did not finish"),
      }
    };
  }

  #[test]
  fn test_decode_ok() {
    assert_eq!(decode_ok(&[
        0b01111000, 0b10011100, 0b01100011, 0b01100100, 0b01100010,
        0b00000110, 0b00000000, 0b00000000, 0b00001101, 0b00000000,
        0b00000111
      ]),
      ~[1, 2, 3]
    );

    assert_eq!(decode_ok(&[
        0b01111000, 0b10011100, 0b01100011, 0b01100010, 0b01100110,
        0b01100101, 0b11100111, 0b00000110, 0b00000000, 0b00000000,
        0b01000011, 0b00000000, 0b00011101
      ]),
      ~[2, 3, 5, 7, 11]
    );

    assert_eq!(decode_ok(&[
        0b01111000, 0b10011100, 0b11101011, 0b10101001, 0b01101001,
        0b10011000, 0b00110001, 0b10100001, 0b10100111, 0b10100111,
        0b01100110, 0b01001010, 0b01000011, 0b01001101, 0b01001011,
        0b11000111, 0b10000100, 0b10011110, 0b00111001, 0b00101101,
        0b00001101, 0b00111101, 0b01110011, 0b00000000, 0b01110000,
        0b00101100, 0b00001010, 0b11000101
      ]),
      ~[140, 124, 128, 152, 144, 140, 140, 124,
        148, 128, 124, 132, 136, 144, 140, 156,
        132, 128, 140, 156]
    );
  }

  #[test]
  fn test_decode_err() {
    assert_eq!(decode_err(&[
        0b01111001, 0b10011100, 0b01100011, 0b01100100, 0b01100010,
      ]),
      (~error::BadCompressionMethod(0b1001),
        &[0b01100011, 0b01100100, 0b01100010])
    );

    assert_eq!(decode_err(&[
        0b10001000, 0b10011100, 0b01100011, 0b01100100, 0b01100010,
      ]),
      (~error::WindowTooLong(64 * 1024), 
        &[0b01100011, 0b01100100, 0b01100010])
    );

    assert_eq!(decode_err(&[
        0b01111000, 0b10111011, 0b01100011, 0b01100100, 0b01100010
      ]),
      (~error::DictionaryUsed,
        &[0b01100011, 0b01100100, 0b01100010])
    );

    assert_eq!(decode_err(&[
        0b01111000, 0b10011101, 0b01100011, 0b01100100, 0b01100010,
      ]),
      (~error::BadHeaderChecksum(0b01111000, 0b10011101),
        &[0b01100011, 0b01100100, 0b01100010])
    );

    assert_eq!(decode_err(&[
        0b01111000, 0b10011100, 0b01100111, 0b01100100, 0b01100010,
      ]),
      (~error::InflateError(~inflate::error::BadBlockType(0b11)),
        &[0b01100100, 0b01100010])
    );

    assert_eq!(decode_err(&[
        0b01111000, 0b10011100, 0b01100011, 0b01100010, 0b01100110,
        0b01100101, 0b11100111, 0b00000110, 0b00000000, 0b00000000,
        0b01000011, 0b11100000, 0b00011101, 7, 8, 9,
      ]),
      (~error::BadDataChecksum(
          0b00000000_01000011_00000000_00011101,
          0b00000000_01000011_11100000_00011101
        ), &[7, 8, 9])
    );
  }

  #[test]
  fn test_decode_chunked() {
    for uint::range(1, 10) |chunk_len| {
      assert_eq!(decode_chunked_ok(chunk_len, &[
          0b01111000, 0b10011100, 0b11101011, 0b10101001, 0b01101001,
          0b10011000, 0b00110001, 0b10100001, 0b10100111, 0b10100111,
          0b01100110, 0b01001010, 0b01000011, 0b01001101, 0b01001011,
          0b11000111, 0b10000100, 0b10011110, 0b00111001, 0b00101101,
          0b00001101, 0b00111101, 0b01110011, 0b00000000, 0b01110000,
          0b00101100, 0b00001010, 0b11000101
        ]),
        ~[140, 124, 128, 152, 144, 140, 140, 124,
          148, 128, 124, 132, 136, 144, 140, 156,
          132, 128, 140, 156]
      );
    }

    /*for uint::range(1, 10) |chunk_len| {
      assert_eq!(decode_chunked_ok(chunk_len, &[
          0x78, 0x9c, 0x25, 0x8c, 0xc1, 0x09, 0x00, 0x40, 0x0c, 0xc2,
          0x66, 0x15, 0x1f, 0x2e, 0x90, 0xfd, 0xb9, 0x94, 0xb3, 0x0f,
          0xc1, 0x46, 0xbb, 0x6e, 0x19, 0x04, 0x2d, 0x25, 0x99, 0x6a,
          0x83, 0x0f, 0xe8, 0x45, 0x8c, 0x6a, 0x05, 0x21, 0x35, 0x44,
          0x85, 0x92, 0x2e, 0x87, 0x5c, 0xf4, 0xaf, 0xd7, 0xd2, 0x32,
          0x0b, 0xce, 0xc8, 0xef, 0x01, 0xfe, 0xbb, 0x28, 0x78
        ]),
        ~[99, 103, 99, 103, 103, 97, 103, 116, 116, 97, 116, 103, 97,
          103, 97, 99, 116, 97, 97, 103, 103, 103, 103, 99, 99, 97,
          116, 99, 103, 103, 116, 116, 99, 97, 99, 116, 97, 116, 103,
          116, 99, 99, 116, 97, 99, 116, 116, 116, 103, 97, 97, 97, 97,
          103, 116, 97, 103, 97, 103, 103, 99, 97, 97, 99, 103, 97, 116,
          99, 97, 99, 97, 97, 103, 116, 103, 116, 103, 116, 103, 116, 99,
          99, 103, 103, 116, 103, 116, 99, 97, 103, 99, 116, 97, 103, 99,
          99, 103, 97, 97, 103]
      );
    }*/
  }

}
