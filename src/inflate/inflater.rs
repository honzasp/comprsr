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
          bit_buf: i_bit_buf,
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

  #[test]
  fn test_inflate_bad_block_type() {
    assert_eq!(inflate_err(&[0b110]), (~error::BadBlockType(0b11), &[]));
  }
}
