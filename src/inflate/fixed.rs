use inflate::bits;
use inflate::compressed::*;
use inflate::error;
use inflate::fixed;
use inflate::out;

pub struct BlockState {
  priv phase: BlockPhase,
}

impl BlockState {
  pub fn new() -> BlockState {
    BlockState { phase: LitlenPhase }
  }

  pub fn input(&mut self, bit_reader: &mut bits::BitReader, out: &mut out::Output) ->
    Option<Result<(),~error::Error>>
  {
    loop {
      self.phase = match self.phase {
        LitlenPhase => {
          if bit_reader.has_bits(7) {
            let rev_prefix = bit_reader.read_bits8(5);
            let (base,extra_bits) = decode_rev_prefix(rev_prefix);

            if bit_reader.has_bits(extra_bits) {
              let code: u16 = base + bit_reader.read_rev_bits8(extra_bits) as u16;
              match decode_litlen(code) {
                Ok(litlen) => match litlen {
                  LiteralCode(byte) => {
                    out.send_literal(byte);
                    LitlenPhase
                  },
                  LengthCode(len,0) =>
                    DistPhase(len),
                  LengthCode(len_base,len_extra_bits) =>
                    LenExtraPhase(len_base,len_extra_bits),
                  BlockEndCode =>
                    return Some(Ok(())),
                },
                Err(err) =>
                  return Some(Err(err)),
              }
            } else {
              bit_reader.unread_bits8(5, rev_prefix);
              return None;
            }
          } else {
            return None;
          }
        },
        LenExtraPhase(len_base,len_extra_bits) => {
          if bit_reader.has_bits(len_extra_bits) {
            let extra = bit_reader.read_bits8(len_extra_bits);
            DistPhase(len_base + extra as uint)
          } else {
            return None;
          }
        },
        DistPhase(len) => {
          if bit_reader.has_bits(5) {
            let dist_code = bit_reader.read_bits8(5);
            match decode_dist(dist_code as u16) {
              Ok((dist_base,dist_extra_bits)) =>
                DistExtraPhase(len,dist_base,dist_extra_bits),
              Err(err) =>
                return Some(Err(err)),
            }
          } else {
            return None;
          }
        },
        DistExtraPhase(len,dist_base,dist_extra_bits) => {
          if bit_reader.has_bits(dist_extra_bits) {
            let dist_extra = bit_reader.read_bits16(dist_extra_bits);
            let dist = dist_base + dist_extra as uint;
            out.back_reference(dist, len);
            LitlenPhase
          } else {
            return None;
          }
        }
      }
    }
  }
}

/*
  000.. .. (+ 256)
  0010. .. (+ 272)
  0011. ... (+ 0)
  01... ... (+ 16)
  10... ... (+ 80)
  11000 ... (+ 280)
  11001 .... (+ 144)
  1101. .... (+ 160)
  111.. .... (+ 192)
*/

fn decode_rev_prefix(rev_prefix: u8) -> (u16,uint) {
  fixed::fixed_table[rev_prefix]
}

pub static fixed_table: [(u16,uint), ..32] = [
  (0,3),   (8,3),   (16,3),  (24,3),  (32,3),  (40,3),  (48,3),  (56,3),
  (64,3),  (72,3),  (80,3),  (88,3),  (96,3),  (104,3), (112,3), (120,3),
  (128,3), (136,3), (144,4), (160,4), (176,4), (192,4), (208,4), (224,4),
  (240,4), (256,2), (260,2), (264,2), (268,2), (272,2), (276,2), (280,3),
];

#[cfg(test)]
mod test {
  use inflate::test_helpers::*;
  use std::uint;

  #[test]
  fn test_inflate_fixed() {
    // literals only
    assert_eq!(inflate_ok(&[
        0b11100011, 0b00010010, 0b10010001, 0b00000011, 0b00000000
      ]),
      ~[10, 20, 30]
    );

    // longer data
    assert_eq!(inflate_ok(&[
        0b11111011, 0b10110001, 0b01101010, 0b01101001,
        0b11111010, 0b10110111, 0b01111111, 0b01011110,
        0b11001101, 0b10111011, 0b10101011, 0b00000001
      ]),
      ~[248, 170, 165, 103, 246, 254, 74, 131, 187, 123]
    );

    // simple length-distance pair
    assert_eq!(inflate_ok(&[
        0b10010011, 0b11010011, 0b00000010,
        0b00000010, 0b00001101, 0b00000000
      ]),
      ~[30, 42, 42, 42, 42, 40]
    );

    /* a longer length-distance pair */
    assert_eq!(inflate_ok(&[
        0b11100011, 0b00010010, 0b01010011,
        0b11000100, 0b00001101, 0b10111001,
        0b11000100, 0b00010100, 0b00000001
      ]),
      ~[ 10, 22, 33, 22, 33, 22, 33, 22
       , 33, 22, 33, 22, 33, 22, 33, 22
       , 33, 22, 33, 22, 33, 22, 33, 22
       , 33, 22, 33, 22, 33, 10, 22, 33]
    );

    /* long repetition and then long distance */
    let res = inflate_ok(&[
        0b00010011, 0b10010001, 0b11010011, 0b00110000, 0b10110010,
        0b11100001, 0b00011010, 0b00000101, 0b00100011, 0b00001110,
        0b10001000, 0b10000000, 0b00100010, 0b00011110, 0b00000000
      ]);

    assert_eq!(res.slice(0, 5), &[20,30,40,50,60]);
    for uint::range(5, 505) |i| {
      assert_eq!(res[i], 10);
    }
    assert_eq!(res.slice(505, 510), &[20,30,40,50,60]);
  }

  #[test]
  fn test_inflate_fixed_errors() {
    // the distance is too long (points before the start of input)
    assert_eq!(inflate_err(&[
        0b1110_0011, 0b0001_0010, 0b0000_0011,
        0b0010_0010, 0b0000_0000
      ]),
      ~error::DistanceTooLong(2, 3)
    );
  }
}

