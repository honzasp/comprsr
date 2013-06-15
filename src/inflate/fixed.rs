use inflate::bits;
use inflate::compressed;
use inflate::error;
use inflate::out;

pub struct BlockState {
  priv phase: compressed::BlockPhase,
}

impl BlockState {
  pub fn new() -> BlockState {
    BlockState { phase: compressed::LitlenPhase }
  }

  pub fn input(&mut self, bit_reader: &mut bits::BitReader, out: &mut out::Output) ->
    Option<Result<(),~error::Error>>
  {
    fail!(~"fixed unimplemented")
  }
}
