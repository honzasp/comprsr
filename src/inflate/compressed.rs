use bits;
use inflate::error;
use inflate::out;

pub enum CompressedPhase {
  LitlenPhase,
  LenExtraPhase(uint,uint), /* (base_len,extra_bits) */
  DistPhase(uint), /* (len) */
  DistExtraPhase(uint,uint,uint), /* (len,base_dist,extra_bits) */
  ErrorPhase(~error::Error),
  EndPhase,
}

#[deriving(Eq)]
pub enum LitlenCode {
  LiteralCode(u8),
  LengthCode(uint, uint),
  BlockEndCode,
}

pub struct ComprState<C> {
  priv phase: CompressedPhase,
  priv coder: C,
}

pub trait Coder {
  fn read_litlen_code(&self, bit_reader: &mut bits::BitReader)
    -> Option<uint>;
  fn read_dist_code(&self, bit_reader: &mut bits::BitReader) 
    -> Option<uint>;
}

impl<C: Coder> ComprState<C> {
  pub fn new(coder: C) -> ComprState<C> {
    ComprState {
      phase: LitlenPhase,
      coder: coder
    }
  }

  pub fn input<R: bits::recv::Recv<u8>> (
    self,
    bit_reader: &mut bits::BitReader,
    out: &mut out::Output<R>
  ) 
    -> Either<ComprState<C>, Result<(), ~error::Error>>
  {
    let mut st = self;

    loop {
      let res = match st.phase {
        LitlenPhase => {
          match st.coder.read_litlen_code(bit_reader) {
            Some(code) => match decode_litlen(code) {
                Ok(litlen) => match litlen {
                  LiteralCode(byte) => {
                    out.send_literal(byte);
                    Some(LitlenPhase)
                  },
                  LengthCode(len, 0) =>
                    Some(DistPhase(len)),
                  LengthCode(len_base, len_extra_bits) =>
                    Some(LenExtraPhase(len_base, len_extra_bits)),
                  BlockEndCode => 
                    Some(EndPhase)
                },
                Err(err) =>
                  Some(ErrorPhase(err))
              },
            None => None,
          }
        },
        LenExtraPhase(len_base, len_extra_bits) => {
          if bit_reader.has_bits(len_extra_bits) {
            let extra = bit_reader.read_bits8(len_extra_bits);
            Some(DistPhase(len_base + extra as uint))
          } else {
            None
          }
        },
        DistPhase(len) => {
          match st.coder.read_dist_code(bit_reader) {
            Some(dist_code) => match decode_dist(dist_code) {
              Ok((dist_base, dist_extra_bits)) =>
                Some(DistExtraPhase(len, dist_base, dist_extra_bits)),
              Err(err) =>
                Some(ErrorPhase(err)),
            },
            None => None,
          }
        },
        DistExtraPhase(len, dist_base, dist_extra_bits) => {
          if bit_reader.has_bits(dist_extra_bits) {
            let dist_extra = bit_reader.read_bits16(dist_extra_bits);
            let dist = dist_base + dist_extra as uint;
            match out.back_reference(dist, len) {
              Ok(()) => Some(LitlenPhase),
              Err(err) => Some(ErrorPhase(err)),
            }
          } else {
            None
          }
        },
        EndPhase => {
          return Right(Ok(()))
        },
        ErrorPhase(err) => {
          return Right(Err(err))
        },
      };

      match res {
        None => return Left(st),
        Some(next_phase) => st.phase = next_phase,
      }
    }
  }
}

pub fn decode_litlen(code: uint) -> Result<LitlenCode,~error::Error> {
  if code < 256 {
    Ok(LiteralCode(code as u8))
  } else if code == 256 {
    Ok(BlockEndCode)
  } else if code < 285 {
    if code <= 264 {
      Ok(LengthCode(code as uint - 254, 0))
    } else {
      let rel = code as uint - 261;
      let extra: uint = rel / 4;
      let base: uint = (1<<(extra+2)) + 3 + (rel%4) * (1<<extra);
      Ok(LengthCode(base, extra))
    }
  } else if code == 285 {
    Ok(LengthCode(258, 0))
  } else {
    Err(~error::BadLitlenCode(code))
  }
}

pub fn decode_dist(code: uint) -> Result<(uint,uint),~error::Error> {
  if code < 4 {
    Ok((code + 1, 0))
  } else if code <= 29 {
    let extra = (code - 2) / 2;
    let base = if code % 2 == 0 {
        1 + (1 << (extra+1))
      } else {
        1 + 3 * (1 << extra)
      };
    Ok((base, extra))
  } else {
    Err(~error::BadDistCode(code))
  }
}

#[cfg(test)]
mod test {
  use inflate::test_helpers::*;
  use inflate::compressed::*;

  #[test]
  fn test_decode_litlen() {
    for (&[0u, 10, 100, 255, 200, 135]).iter().advance |&x| {
      assert_eq!(decode_litlen(x), Ok(LiteralCode(x as u8)));
    };

    assert_eq!(decode_litlen(256), Ok(BlockEndCode));

    assert_eq!(decode_litlen(257), Ok(LengthCode(3, 0)));
    assert_eq!(decode_litlen(260), Ok(LengthCode(6, 0)));
    assert_eq!(decode_litlen(265), Ok(LengthCode(11, 1)));
    assert_eq!(decode_litlen(271), Ok(LengthCode(27, 2)));
    assert_eq!(decode_litlen(274), Ok(LengthCode(43, 3)));
    assert_eq!(decode_litlen(280), Ok(LengthCode(115, 4)));
    assert_eq!(decode_litlen(282), Ok(LengthCode(163, 5)));
    assert_eq!(decode_litlen(285), Ok(LengthCode(258, 0)));

    for (&[286u, 287, 300, 1024]).iter().advance |&y| {
      assert_eq!(decode_litlen(y), Err(~error::BadLitlenCode(y)));
    };
  }

  #[test]
  fn test_decode_dist() {
    assert_eq!(decode_dist(0), Ok((1, 0)));
    assert_eq!(decode_dist(3), Ok((4, 0)));
    assert_eq!(decode_dist(5), Ok((7, 1)));
    assert_eq!(decode_dist(6), Ok((9, 2)));
    assert_eq!(decode_dist(12), Ok((65, 5)));
    assert_eq!(decode_dist(15), Ok((193, 6)));
    assert_eq!(decode_dist(18), Ok((513, 8)));
    assert_eq!(decode_dist(23), Ok((3073, 10)));
    assert_eq!(decode_dist(24), Ok((4097, 11)));
    assert_eq!(decode_dist(29), Ok((24577, 13)));

    for (&[30u, 31, 40, 50, 1000]).iter().advance |&x| {
      assert_eq!(decode_dist(x), Err(~error::BadDistCode(x)));
    };
  }
}
