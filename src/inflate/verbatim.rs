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
  BeginDataPhase,
  DataPhase,
}

impl BlockState {
  pub fn new() -> BlockState {
    BlockState { phase: BeginPhase, len: 0, nlen: 0, remaining: 0 }
  }

  pub fn input(&mut self, bit_reader: &mut bits::BitReader, out: &mut out::Output)
    -> Option<Result<(),~error::Error>>
  {
    loop {
      self.phase = match self.phase {
        BeginPhase => {
          bit_reader.skip_to_byte();
          LenPhase
        },
        LenPhase => {
          if bit_reader.has_bytes(2) { 
            self.len = bit_reader.read_u16();
            NLenPhase
          } else { return None }
        }
        NLenPhase => {
          if bit_reader.has_bytes(2) {
            self.nlen = bit_reader.read_u16();
            BeginDataPhase
          } else { return None }
        },
        BeginDataPhase => {
          if self.len == !self.nlen {
            self.remaining = self.len as uint;
            DataPhase
          } else {
            return Some(Err(~error::VerbatimLengthMismatch(self.len, self.nlen)));
          }
        },
        DataPhase => {
          let chunk = bit_reader.read_byte_chunk(self.remaining);
          out.send_literal_chunk(chunk);

          if chunk.len() < self.remaining {
            self.remaining -= chunk.len();
            DataPhase
          } else {
            return Some(Ok(()));
          }
        }
      }
    }
  }
}
