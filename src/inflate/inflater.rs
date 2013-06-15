use inflate::bits;
use inflate::dynamic;
use inflate::error;
use inflate::fixed;
use inflate::inflater;
use inflate::out;
use inflate::verbatim;

pub struct Inflater<'self> {
  priv stage: Stage,
  priv bit_buf: bits::BitBuf,
  priv output: ~out::Output<'self>,
  priv last_block: bool,
}

pub enum Res<'self> {
  pub ConsumedRes(),
  pub FinishedRes(&'self [u8]),
  pub ErrorRes(~error::Error, &'self [u8]),
}

enum Stage {
  EndStage,
  ErrorStage(~error::Error),
  HeaderStage,
  DynamicHeaderStage(~dynamic::HeaderState),
  VerbatimStage(~verbatim::BlockState),
  FixedStage(~fixed::BlockState),
  DynamicStage(~dynamic::BlockState),
}

pub static window_size: uint = 32_768;

impl<'self> Inflater<'self> {
  pub fn new<'a>(callback: &'a fn(&[u8])) -> Inflater<'a> {
    Inflater {
      stage: HeaderStage,
      bit_buf: bits::BitBuf::new(),
      output: ~out::Output::new(callback, inflater::window_size),
      last_block: false,
    }
  }

  pub fn input<'a>(&mut self, chunk: &'a [u8]) -> Res<'a> {
    let mut bit_reader = ~bits::BitReader::new(&self.bit_buf, chunk);

    loop {
      self.stage = match self.stage {
        EndStage => 
          return FinishedRes(bit_reader.unconsumed_bytes(chunk)),
        ErrorStage(ref err) => 
          return ErrorRes(err.clone(), bit_reader.unconsumed_bytes(chunk)),
        HeaderStage if self.last_block => 
          EndStage,
        HeaderStage => {
          if !bit_reader.has_bits(3) { break; }

          let bfinal = bit_reader.read_bits8(1);
          let btype = bit_reader.read_bits8(2);

          self.last_block = bfinal != 0;
          match btype {
            0b00 => VerbatimStage(~verbatim::BlockState::new()),
            0b01 => FixedStage(~fixed::BlockState::new()),
            0b10 => DynamicHeaderStage(~dynamic::HeaderState::new()),
            _    => ErrorStage(~error::BadBlockType(btype)),
          }
        },
        DynamicHeaderStage(ref mut dyn_hdr_state) => 
          match dyn_hdr_state.input(bit_reader) {
            Some(Err(err)) => ErrorStage(err),
            Some(Ok(dyn_block_state)) =>
              DynamicStage(dyn_block_state),
            None => break,
          },
        _ => {
          let result = match self.stage {
            DynamicStage(ref mut dyn_state) =>
              dyn_state.input(bit_reader, self.output),
            FixedStage(ref mut fixed_state) =>
              fixed_state.input(bit_reader, self.output),
            VerbatimStage(ref mut verb_state) =>
              verb_state.input(bit_reader, self.output),
            _ => fail!(~"unreachable"),
          };

          match result {
            Some(Err(err)) => ErrorStage(err),
            Some(Ok(()))   => HeaderStage,
            None           => break,
          }
        },
      }
    }

    self.bit_buf = bit_reader.rest_bit_buf();
    ConsumedRes
  }

  pub fn is_finished(&self) -> bool {
    match self.stage {
      EndStage      => true,
      ErrorStage(_) => true,
      _ => false,
    }
  }

  pub fn get_error(&self) -> Option<~error::Error> {
    match self.stage {
      ErrorStage(ref err) => Some(err.clone()),
      _ => None
    }
  }

  pub fn is_error(&self) -> bool {
    self.get_error().is_some()
  }

  pub fn is_ready(&self) -> bool {
    !self.is_finished()
  }
}
