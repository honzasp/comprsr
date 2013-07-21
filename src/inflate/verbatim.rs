use bits;
use inflate::error;
use inflate::out;

pub struct VerbState {
  priv phase: VerbPhase,
  priv len: u16,
  priv nlen: u16,
}

enum VerbPhase {
  BeginPhase(),
  LenPhase(),
  NLenPhase(),
  BeginDataPhase(),
  DataPhase(uint),
  EndPhase(),
  ErrorPhase(~error::Error),
}

impl VerbState {
  pub fn new() -> VerbState {
    VerbState { phase: BeginPhase, len: 0, nlen: 0 }
  }

  pub fn input <R: bits::recv::Recv<u8>> (
    self,
    bit_reader: &mut bits::BitReader,
    out: &mut out::Output,
    recv: R
  )
    -> (Either<VerbState, Result<(), ~error::Error>>, R)
  {
    let mut st = self;
    let mut recv = recv;

    loop {
      let (continue, next_phase) = match st.phase {
        BeginPhase() => {
          bit_reader.skip_to_byte();
          (true, LenPhase)
        },
        LenPhase() => {
          if bit_reader.has_bytes(2) { 
            st.len = bit_reader.read_u16();
            (true, NLenPhase)
          } else { 
            (false, LenPhase)
          }
        }
        NLenPhase() => {
          if bit_reader.has_bytes(2) {
            st.nlen = bit_reader.read_u16();
            (true, BeginDataPhase)
          } else {
            (false, NLenPhase) 
          }
        },
        BeginDataPhase() => {
          if st.len == !st.nlen {
            (true, DataPhase(st.len as uint))
          } else {
            (true, ErrorPhase(~error::VerbatimLengthMismatch(st.len, st.nlen)))
          }
        },
        DataPhase(remaining) => {
          let chunk = bit_reader.read_byte_chunk(remaining);
          recv = out.send_literal_chunk(chunk, recv);

          if chunk.len() < remaining {
            (false, DataPhase(remaining - chunk.len()))
          } else {
            (true, EndPhase)
          }
        },
        EndPhase() => {
          return (Right(Ok(())), recv)
        },
        ErrorPhase(err) => {
          return (Right(Err(err)), recv)
        }
      };

      st.phase = next_phase;
      if !continue {
        return (Left(st), recv)
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
      let mut inflater = inflater::Inflater::new();
      let mut input_pos = 0;
      let mut finished = false;

      while input_pos < bytes.len() {
        let next_pos = cmp::min(bytes.len(), input_pos + 1024);
        match inflater.input(bytes.slice(input_pos, next_pos), ()) {
          (Left(new_infl), ()) => inflater = new_infl,
          (Right((Ok(()), [])), ()) => { finished = true; break },
          other => fail!(fmt!("Unexpected result %?", other)),
        };
        input_pos = next_pos;
      }

      assert!(finished);
    };
  }

}
