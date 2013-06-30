use bits;
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

  pub fn input<R: bits::recv::Receiver<u8>>(
    &mut self,
    bit_reader: &mut bits::BitReader,
    out: &mut out::Output<R>
  )
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
  use extra::test;
  use std::rand;
  use std::rand::{RngUtil};
  use std::cmp;

  use inflate::test_helpers::*;
  use inflate::inflater;

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
      (~error::VerbatimLengthMismatch(
        0b0000_0000_0000_0101, 0b1111_1111_1100_0000
      ), &[]));
  }

  /* TODO: this isn't possible with Receivers
  #[test]
  fn test_inflate_verbatim_chunks() {
    let mut buf: ~[u8] = ~[];
    let mut inflater = inflater::Inflater::new(&mut buf);

    inflater.input(&[0b00000_000, 0b00001010]);
    assert!(buf.is_empty());
    inflater.input(&[0b00000000, 0b11110101, 0b11111111]);
    assert!(buf.is_empty());
    inflater.input(&[10,20,30,40,50]);
    assert_eq!(&buf, &~[10,20,30,40,50]);
  }
  */

  #[bench]
  fn bench_verbatim(b: &mut test::BenchHarness) {
    fn gen_verb_block<R: rand::Rng>(
      bytes: &mut ~[u8],
      len: u16,
      final: bool,
      rng: &mut R
    ) {
      bytes.push(if final { 0b00000_001 } else { 0b00000_000 });
      bytes.push((len & 0xff) as u8);
      bytes.push((len >> 8) as u8);
      bytes.push(!(len & 0xff) as u8);
      bytes.push(!(len >> 8) as u8);

      bytes.push_all(rng.gen_bytes(len as uint));
    }

    let mut bytes: ~[u8] = ~[];
    {
      let mut remaining_len = 12_456;
      let rng = &mut rand::rng();

      for 10.times {
        let len = rng.gen_uint_range(900, 1200) as u16;
        gen_verb_block(&mut bytes, len, false, rng);
        remaining_len -= len;
      }

      gen_verb_block(&mut bytes, remaining_len, true, rng);
    }

    do b.iter {
      let mut inflater = inflater::Inflater::new(~());
      let mut input_pos = 0;

      while input_pos < bytes.len() {
        let next_pos = cmp::min(bytes.len(), input_pos + 1024);
        match inflater.input(bytes.slice(input_pos, next_pos)) {
          inflater::ConsumedRes => { },
          inflater::FinishedRes(rest) if rest.is_empty() => { },
          other => fail!(fmt!("Unexpected res %?", other)),
        };
        input_pos = next_pos;
      }

      assert!(inflater.has_finished());
    };
  }

}
