use inflate::bits;
use inflate::error;
use inflate::out;

pub struct BlockState {
  priv phase: BlockPhase,
  priv len: u16,
  priv nlen: u16,
  priv remaining: uint,
}

enum BlockPhase {
  BeginPhase,
  LenPhase,
  NLenPhase,
  BlockPhase,
}

impl BlockState {
  pub fn new() -> BlockState {
    BlockState { phase: BeginPhase, len: 0, nlen: 0, remaining: 0 }
  }

  pub fn input(&mut self, bit_reader: &mut bits::BitReader, out: &mut out::Output)
    -> Option<Result<(),~error::Error>>
  {
    fail!(~"verbatim unimplemented")
  }
}
