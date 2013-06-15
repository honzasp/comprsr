use inflate::bits;
use inflate::compressed;
use inflate::error;
use inflate::huff;

pub struct HeaderState {
  priv phase: HeaderPhase,
  priv meta_count: uint,
  priv meta_lens: ~[u8],
  priv meta_tree: ~huff::Tree,
  priv litlen_count: uint,
  priv dist_count: uint,
  priv code_lens: ~[u8],
}

enum HeaderPhase {
  NumbersPhase,
  MetaLensPhase,
  CodeLensPhase,
  EndPhase,
}

pub struct BlockState {
  priv phase: compressed::BlockPhase,
  priv litlen_tree: ~huff::Tree,
  priv dist_tree: ~huff::Tree,
}

impl HeaderState {
  pub fn new() -> HeaderState {
    HeaderState {
      phase: NumbersPhase,
      meta_count: 0, meta_lens: ~[], meta_tree: ~huff::Tree::new(),
      litlen_count: 0, dist_count: 0, code_lens: ~[],
    }
  }

  pub fn input(&mut self, bit_reader: &mut bits::BitReader)
    -> Option<Result<~BlockState,~error::Error>>
  {
    fail!(~"dynamic header unimplemented")
  }
}

impl BlockState {
  pub fn new(hdr_state: &HeaderState) -> BlockState {
    fail!(~"dynamic block new unimplemented")
  }

  pub fn input(&mut self, bit_reader: &mut bits::BitReader)
    -> Option<Result<(),~error::Error>>
  {
    fail!(~"dynamic block unimplemented")
  }
}
