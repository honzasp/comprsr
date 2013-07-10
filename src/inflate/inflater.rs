use bits;
use inflate::dynamic;
use inflate::error;
use inflate::fixed;
use inflate::inflater;
use inflate::out;
use inflate::verbatim;
use inflate::compressed;

pub struct Inflater<R> {
  priv stage: Stage,
  priv bit_buf: bits::BitBuf,
  priv output: ~out::Output<R>,
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

impl<R: bits::recv::Recv<u8>> Inflater<R> {
  pub fn new(receiver: R) -> Inflater<R> {
    Inflater {
      stage: HeaderStage,
      bit_buf: bits::BitBuf::new(),
      output: ~out::Output::new(inflater::window_size, receiver),
      last_block: false,
    }
  }

  pub fn close(self) -> R {
    self.output.close()
  }

  pub fn input<'a>(self, chunk: &'a [u8])
    -> Either<Inflater<R>, (Result<R, ~error::Error>, &'a [u8])>
  {
    let reader_res = do bits::BitReader::with_buf(&mut i_bit_buf, chunk)
      |bit_reader| 
    {
      let Inflater { stage, bit_buf, output, last_block } = self;
      let mut i_bit_buf = bit_buf;
      let mut i_output = output;
      let mut i_last_block = last_block;
      let mut i_stage = stage;

      // TODO: Rust doesn't allow to use `return` in "block function"
      let mut ret;

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
            match dyn_hdr_state.input(bit_reader) {
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
            match verb_state.input(bit_reader, i_output) {
              Left(new_state) => (false, VerbatimStage(new_state)),
              Right(Ok(()))   => (true, HeaderStage),
              Right(Err(err)) => (true, ErrorStage(err)),
            }
          },
          FixedStage(compr_state) => {
            match compr_state.input(bit_reader, i_output) {
              Left(new_state) => (false, FixedStage(new_state)),
              Right(Ok(()))   => (true, HeaderStage),
              Right(Err(err)) => (true, ErrorStage(err)),
            }
          },
          DynamicStage(compr_state) => {
            match compr_state.input(bit_reader, i_output) {
              Left(new_state) => (false, DynamicStage(new_state)),
              Right(Ok(()))   => (true, HeaderStage),
              Right(Err(err)) => (true, ErrorStage(err)),
            }
          },
          EndStage => {
            //return Some(Ok(()))
            ret = Right(Ok(i_output.close())); break
          },
          ErrorStage(err) => {
            //return Some(Err(err))
            ret = Right(Err(err)); break
          },
        };

        i_stage = new_stage;
        if !continue { 
          ret = Left(Inflater {
            stage: i_stage,
            bit_buf: i_bit_buf,
            output: i_output,
            last_block: i_last_block
          }); break
        }
      }

      ret
    };

    match reader_res {
      Left(inflater) => Left(inflater),
      Right((Ok(recv), rest))   => Right((Ok(recv), rest)),
      Right((Err(err), rest)) => Right((Err(err), rest)),
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
  use inflate::inflater;
  use inflate::test_helpers::*;

  #[test]
  fn test_inflate_bad_block_type() {
    assert_eq!(inflate_err(&[0b110]), (~error::BadBlockType(0b11), &[]));
  }

  #[test]
  fn test_inflate_close() {
    let buf: ~[u8] = ~[];

    let mut inflater = inflater::Inflater::new(buf);
    inflater.input(&[
        0b00000_001,
        0b00001010, 0b00000000,
        0b11110101, 0b11111111,
        10, 20, 30, 40, 50, 60, 70, 80, 90, 100
      ]);

    let buf = inflater.close();
    assert_eq!(buf, ~[10, 20, 30, 40, 50, 60, 70, 80, 90, 100]);
  }
}
