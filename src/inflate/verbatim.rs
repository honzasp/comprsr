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
            return None
          } else {
            return Some(Ok(()));
          }
        }
      }
    }
  }
}

#[cfg(test)]
mod test {
  use inflate::test_helpers::*;

  #[test]
  fn test_inflate_verbatim() {
    // one block 
    assert_eq!(inflate_ok(&[
        0b00000_001,
        0b00001010, 0b00000000,
        0b11110101, 0b11111111,
        10, 20, 30, 40, 50, 60, 70, 80, 90, 100
      ]),
      ~[10, 20, 30, 40, 50, 60, 70, 80, 90, 100]
    );

    // two blocks 
    assert_eq!(inflate_ok(&[
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

    // empty block
    assert_eq!(inflate_ok(&[
        0b00000_001,
        0b00000000, 0b00000000,
        0b11111111, 0b11111111,
      ]),
      ~[]
    );
  }

  #[test]
  fn test_inflate_verbatim_errors() {
    // the length and the inverse don't match 
    assert_eq!(inflate_err(&[
        0b00000_001,
        0b0000_0101, 0b0000_0000,
        0b1100_0000, 0b1111_1111
      ]),
      ~error::VerbatimLengthMismatch(
        0b0000_0000_0000_0101, 0b1111_1111_1100_0000
      ));
  }

  #[test]
  fn test_inflate_verbatim_chunks() {
    let mut buf = ~[];
    let mut inflater = do inflater::Inflater::new |chunk| {
        buf.push_all(chunk)
      };

    inflater.input(&[0b00000_000, 0b00001010]);
    assert!(buf.is_empty());
    inflater.input(&[0b00000000, 0b11110101, 0b11111111]);
    assert!(buf.is_empty());
    inflater.input(&[10,20,30,40,50]);
    assert_eq!(&buf, &~[10,20,30,40,50]);
  }
}
