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
