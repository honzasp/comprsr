use bits;
use inflate::dynamic;
use inflate::error;
use inflate::fixed;
use inflate::inflater;
use inflate::out;
use inflate::verbatim;
use inflate::compressed;

pub struct Inflater {
  priv stage: Stage,
  priv bit_buf: bits::BitBuf,
  priv output: ~out::Output,
  priv last_block: bool,
}

enum Stage {
  HeaderStage,
  DynamicHeaderStage(dynamic::HeaderState),
  VerbatimStage(verbatim::VerbState),
  FixedStage(compressed::ComprState<fixed::FixedCoder>),
  DynamicStage(compressed::ComprState<dynamic::DynamicCoder>),
  EndStage,
  ErrorStage(~error::Error),
}

pub static window_size: uint = 32_768;

impl Inflater {
  pub fn new() -> Inflater {
    Inflater {
      stage: HeaderStage,
      bit_buf: bits::BitBuf::new(),
      output: ~out::Output::new(inflater::window_size),
      last_block: false,
    }
  }

  pub fn input<'a, R: bits::recv::Recv<u8>>
    (self, chunk: &'a [u8], recv: R)
    -> (Either<Inflater, (Result<(), ~error::Error>, &'a [u8])>, R)
  {
    // TODO: make a single variable and access its members?
    let Inflater { stage, bit_buf, output, last_block } = self;
    let i_bit_buf = bit_buf;
    let mut i_output = output;
    let mut i_last_block = last_block;
    let mut i_stage = stage;

    let mut recv = recv;
    let mut bit_reader = bits::BitReader::new(i_bit_buf, chunk);

    loop {
      let (continue, new_stage) = match i_stage {
        HeaderStage if i_last_block => 
          (true, EndStage),
        HeaderStage => {
          if bit_reader.has_bits(3) {
            let bfinal = bit_reader.read_bits8(1);
            let btype = bit_reader.read_bits8(2);

            i_last_block = bfinal != 0;
            (true, match btype {
              0b00 => VerbatimStage(verbatim::VerbState::new()),
              0b01 => FixedStage(compressed::ComprState::new(fixed::FixedCoder::new())),
              0b10 => DynamicHeaderStage(dynamic::HeaderState::new()),
              _    => ErrorStage(~error::BadBlockType(btype as uint)),
            })
          } else {
            (false, HeaderStage)
          }
        },
        DynamicHeaderStage(dyn_hdr_state) => {
          match dyn_hdr_state.input(&mut bit_reader) {
            Left(new_state) => 
              (false, DynamicHeaderStage(new_state)),
            Right(Err(err)) =>
              (true, ErrorStage(err)),
            Right(Ok(dyn_coder)) =>
              (true, DynamicStage(compressed::ComprState::new(dyn_coder))),
          }
        },
        // TODO: make it dryer!
        VerbatimStage(verb_state) => {
          let (res, new_recv) = verb_state.input(&mut bit_reader, i_output, recv);
          recv = new_recv;
          match res {
            Left(new_state) => (false, VerbatimStage(new_state)),
            Right(Ok(()))   => (true, HeaderStage),
            Right(Err(err)) => (true, ErrorStage(err)),
          }
        },
        FixedStage(compr_state) => {
          let (res, new_recv) = compr_state.input(&mut bit_reader, i_output, recv);
          recv = new_recv;
          match res {
            Left(new_state) => (false, FixedStage(new_state)),
            Right(Ok(()))   => (true, HeaderStage),
            Right(Err(err)) => (true, ErrorStage(err)),
          }
        },
        DynamicStage(compr_state) => {
          let (res, new_recv) = compr_state.input(&mut bit_reader, i_output, recv);
          recv = new_recv;
          match res {
            Left(new_state) => (false, DynamicStage(new_state)),
            Right(Ok(()))   => (true, HeaderStage),
            Right(Err(err)) => (true, ErrorStage(err)),
          }
        },
        EndStage => {
          let (_bit_buf, rest_bytes) = bit_reader.close();
          return (Right((Ok(()), rest_bytes)), recv)
        },
        ErrorStage(err) => {
          let (_bit_buf, rest_bytes) = bit_reader.close();
          return (Right((Err(err), rest_bytes)), recv)
        },
      };

      i_stage = new_stage;
      if !continue { 
        return (Left(Inflater {
          stage: i_stage,
          bit_buf: bit_reader.close_to_buf(),
          output: i_output,
          last_block: i_last_block
        }), recv)
      }
    }
  }

  pub fn has_finished(&self) -> bool {
    match self.stage {
      EndStage      => true,
      ErrorStage(_) => true,
      _ => false,
    }
  }
}

#[cfg(test)]
mod test {
  use inflate::test_helpers::*;
  use std::uint;

  #[test]
  fn test_inflate_bad_block_type() {
    assert_eq!(inflate_err(&[0b110]), (~error::BadBlockType(0b11), &[]));
  }

  #[test]
  fn test_inflate_chunked() {
    for uint::range(1, 10) |chunk_len| {
      // verbatim
      assert_eq!(inflate_chunked_ok(chunk_len, &[
          0b00000_000,
          0b0000_0110, 0b0000_0000,
          0b1111_1001, 0b1111_1111,
          11, 22, 33, 44, 55, 66,
          0b00000_001,
          0b0000_0100, 0b0000_0000,
          0b1111_1011, 0b1111_1111,
          77, 88, 99, 110
        ]), 
        ~[11, 22, 33, 44, 55, 66, 77, 88, 99, 110]
      );

      // fixed
      assert_eq!(inflate_chunked_ok(chunk_len, &[
          0b11100011, 0b00010010, 0b01010011,
          0b11000100, 0b00001101, 0b10111001,
          0b11000100, 0b00010100, 0b00000001
        ]),
        ~[ 10, 22, 33, 22, 33, 22, 33, 22
        , 33, 22, 33, 22, 33, 22, 33, 22
        , 33, 22, 33, 22, 33, 22, 33, 22
        , 33, 22, 33, 22, 33, 10, 22, 33]
      );

      // dynamic
      assert_eq!(inflate_chunked_ok(chunk_len, &[
         0b00001101, 0b11001000, 0b00110001, 0b00010001, 0b00000000,
         0b00000000, 0b00001100, 0b00000010, 0b00110001, 0b01000100,
         0b01110000, 0b10001100, 0b11101000, 0b01111100, 0b11110111,
         0b10110100, 0b00011001, 0b01010011, 0b01001000, 0b00100111,
         0b10001111, 0b01111001, 0b00000011, 0b00100101, 0b00111111,
         0b00110110, 0b01010110, 0b11001010, 0b00000001 
        ]),
        ~[30, 120, 120, 22, 30, 255, 0, 20, 255,
          120, 255, 20, 255, 255, 120, 120, 0, 22,
          22, 120, 120, 22, 20, 20, 120, 20, 0, 22,
          30, 120]
      );
    }

  }
}
