use recv;
use inflate::bits;
use inflate::dynamic;
use inflate::error;
use inflate::fixed;
use inflate::inflater;
use inflate::out;
use inflate::verbatim;

pub struct Inflater<'self, R> {
  priv stage: Stage,
  priv bit_buf: bits::BitBuf,
  priv output: ~out::Output<R>,
  priv last_block: bool,
}

#[deriving(Eq)]
pub enum Res<A> {
  pub ConsumedRes(),
  pub FinishedRes(A),
  pub ErrorRes(~error::Error, A),
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

impl<R: recv::Receiver<u8>> Inflater<R> {
  pub fn new(receiver: ~R) -> Inflater<R> {
    Inflater {
      stage: HeaderStage,
      bit_buf: bits::BitBuf::new(),
      output: ~out::Output::new(inflater::window_size, receiver),
      last_block: false,
    }
  }

  pub fn finish(self) -> ~R {
    self.output.finish()
  }

  pub fn input<'a>(&mut self, chunk: &'a [u8]) -> Res<&'a [u8]> {
    do bits::BitReader::with_buf(&mut self.bit_buf, chunk) |bit_reader| {
      // TODO: Rust doesn't support `return` from lambdas !!!
      let mut ret = None;

      loop {
        self.stage = match self.stage {
          EndStage => { 
            ret = Some(Ok(())); break },
          ErrorStage(ref err) => {
            ret = Some(Err(err.clone())); break },
          HeaderStage if self.last_block => 
            EndStage,
          HeaderStage => {
            if bit_reader.has_bits(3) {
              let bfinal = bit_reader.read_bits8(1);
              let btype = bit_reader.read_bits8(2);

              self.last_block = bfinal != 0;
              match btype {
                0b00 => VerbatimStage(~verbatim::BlockState::new()),
                // TODO: make it fixed::BlockState::new()
                0b01 => FixedStage(~fixed::new_block_state()),
                0b10 => DynamicHeaderStage(~dynamic::HeaderState::new()),
                _    => ErrorStage(~error::BadBlockType(btype as uint)),
              }
            } else {
              break
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

      self.output.flush();
      ret
    }
  }

  pub fn has_finished(&self) -> bool {
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
    !self.has_finished()
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
  fn test_inflate_finish() {
    let buf: ~[u8] = ~[];

    let mut inflater = inflater::Inflater::new(~buf);
    inflater.input(&[
        0b00000_001,
        0b00001010, 0b00000000,
        0b11110101, 0b11111111,
        10, 20, 30, 40, 50, 60, 70, 80, 90, 100
      ]);

    let buf = *inflater.finish();
    assert_eq!(buf, ~[10, 20, 30, 40, 50, 60, 70, 80, 90, 100]);
  }
}
