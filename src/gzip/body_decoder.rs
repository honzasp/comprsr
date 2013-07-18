use checksums::crc32;
use bits;
use bits::recv;
use gzip::error;
use inflate::inflater;

pub struct BodyDecoder {
  priv stage: Stage,
  priv byte_buf: bits::ByteBuf,
}

enum Stage {
  DataStage(inflater::Inflater, crc32::Crc32, u32),
  Crc32Stage(u32, u32),
  ISizeStage(u32),
  ErrorStage(~error::Error),
  EndStage(),
}

impl BodyDecoder {
  pub fn new() -> BodyDecoder {
    BodyDecoder { 
      stage: DataStage(inflater::Inflater::new(), crc32::Crc32::new(), 0),
      byte_buf: bits::ByteBuf::new(),
    }
  }

  pub fn input<'a, R: recv::Recv<u8>>(self, chunk: &'a [u8], recv: R)
    -> (Either<BodyDecoder, (Result<(), ~error::Error>, &'a [u8])>, R)
  {
    let BodyDecoder { stage, byte_buf } = self;
    let mut stage = stage;
    let mut recv = recv;
    let mut byte_reader = bits::ByteReader::new(byte_buf, chunk);

    loop {
      let (continue, new_stage) = match stage {
        DataStage(inflater, crc, isize) => {
          let (new_recv, continue, new_stage) = BodyDecoder::data_stage(
            inflater, crc, isize, recv, &mut byte_reader);
          recv = new_recv;
          (continue, new_stage)
        },
        Crc32Stage(computed_crc, isize) =>
          BodyDecoder::crc32_stage(computed_crc, isize, &mut byte_reader),
        ISizeStage(isize) => 
          BodyDecoder::isize_stage(isize, &mut byte_reader),
        ErrorStage(err) => 
          return (Right((Err(err), byte_reader.close_to_rest())), recv),
        EndStage => 
          return (Right((Ok(()), byte_reader.close_to_rest())), recv),
      };

      if !continue {
        let byte_buf = byte_reader.close_to_buf();
        let decoder = BodyDecoder { stage: new_stage, byte_buf: byte_buf };
        return (Left(decoder), recv)
      } else {
        stage = new_stage;
      }
    }
  }

  fn data_stage<R: recv::Recv<u8>>(
    inflater: inflater::Inflater, crc: crc32::Crc32, isize: u32,
    recv: R, byte_reader: &mut bits::ByteReader)
    -> (R, bool, Stage)
  {
    if byte_reader.has_some_bytes() {
      do byte_reader.consume_chunk((inflater, crc, isize, recv))
        |(inflater, crc, isize, recv), chunk|
      {
        let (res, (n_recv, n_crc, n_isize)) =
          inflater.input(chunk, (recv, crc, isize));

        match res {
          Left(n_inflater) =>
            ((n_recv, false, DataStage(n_inflater, n_crc, n_isize)), None),
          Right((Ok(()), rest)) =>
            ((n_recv, true, Crc32Stage(n_crc.crc32(), n_isize)), Some(rest)),
          Right((Err(err), rest)) =>
            ((n_recv, true, ErrorStage(~error::InflateError(err))), Some(rest)),
        }
      }
    } else {
      (recv, false, DataStage(inflater, crc, isize))
    }
  }

  fn crc32_stage(computed_crc: u32, isize: u32,
    byte_reader: &mut bits::ByteReader) -> (bool, Stage) 
  {
    if byte_reader.has_bytes(4) {
      let read_crc = byte_reader.read_u32_le();
      if read_crc == computed_crc {
        (true, ISizeStage(isize))
      } else {
        (true, ErrorStage(~error::BadDataChecksum(computed_crc, read_crc)))
      }
    } else {
      (false, Crc32Stage(computed_crc, isize))
    }
  }

  fn isize_stage(isize: u32, byte_reader: &mut bits::ByteReader) 
    -> (bool, Stage) 
  {
    if byte_reader.has_bytes(4) {
      let read_isize = byte_reader.read_u32_le();
      if read_isize == isize {
        (true, EndStage)
      } else {
        (true, ErrorStage(~error::BadDataSize
          (isize as uint, read_isize as uint)))
      }
    } else {
      (false, ISizeStage(isize))
    }
  }
}

#[cfg(test)]
mod test {
  use gzip::test_helpers::*;
  use gzip::error;
  use std::uint;

  #[test]
  fn test_decode_body_ok() {
    assert_eq!(decode_body_ok(&[
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

  #[test]
  fn test_decode_body_chunked() {
    for uint::range(1, 10) |chunk_len| {
      assert_eq!(decode_body_chunked_ok(chunk_len, &[
          0x63, 0x64, 0x64, 0x62, 0x66, 0xe5,
          0xe0, 0x15, 0x55, 0x32, 0x07, 0x00,
          0xf0, 0x8a, 0xcb, 0xff,
          0x0a, 0x00, 0x00, 0x00,
        ]),
        ~[1, 1, 2, 3, 5, 8, 13, 21, 34, 55]);
    }
  }
}
