use bits;
use inflate::compressed;

pub type BlockState = compressed::BlockState<BlockExtra>;
struct BlockExtra();

impl compressed::BlockExtra for BlockExtra {
  fn read_litlen_code(&self, bit_reader: &mut bits::BitReader)
    -> Option<uint>
  {
    read_fixed_code(bit_reader)
  }
  
  fn read_dist_code(&self, bit_reader: &mut bits::BitReader) 
    -> Option<uint>
  {
    read_fixed_dist_code(bit_reader)
  }
}

// TODO: impl BlockState { fn new() }
// (was blocked by a compiler error which seems to be incorrect)
pub fn new_block_state() -> BlockState {
  compressed::BlockState {
    phase: compressed::LitlenPhase,
    extra: BlockExtra,
  }
}

fn read_fixed_code(bit_reader: &mut bits::BitReader) -> Option<uint> {
  if bit_reader.has_bits(7) {
    let rev_prefix = bit_reader.read_bits8(5);
    let (base,extra_bits) = decode_rev_prefix(rev_prefix as uint);

    if bit_reader.has_bits(extra_bits) {
      let code = base + bit_reader.read_rev_bits8(extra_bits) as uint;
      Some(code)
    } else {
      bit_reader.unread_bits8(5, rev_prefix);
      None
    }
  } else {
    None
  }
}

fn read_fixed_dist_code(bit_reader: &mut bits::BitReader) -> Option<uint> {
  if bit_reader.has_bits(5) {
    let code = bit_reader.read_rev_bits8(5);
    Some(code as uint)
  } else {
    None
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

fn decode_rev_prefix(rev_prefix: uint) -> (uint, uint) {
  fn rev2(x: uint) -> uint {
    ((x & 0b10) >> 1) | ((x & 0b01) << 1)
  }

  fn rev3(x: uint) -> uint {
    ((x & 0b100) >> 2) | (x & 0b010) | ((x & 0b001) << 2)
  }

  match rev_prefix & 0b11 {
    0b00 => 
      match (rev_prefix & 0b1100) >> 2 {
        0b00 | 0b10 => (256 + (rev2((rev_prefix & 0b11000) >> 3) << 2), 2),
        0b01 => (272 + ((rev_prefix & 0b10000) >> 2), 2),
        0b11 => (0 + ((rev_prefix & 0b10000) >> 1), 3),
        _ => fail!(~"unreachable"),
      },
    0b10 => (16 + (rev3((rev_prefix & 0b11100) >> 2) << 3), 3),
    0b01 => (80 + (rev3((rev_prefix & 0b11100) >> 2) << 3), 3),
    0b11 => 
      match (rev_prefix & 0b1100) >> 2 {
        0b00 =>
          match (rev_prefix & 0b10000) >> 4 {
            0b0 => (280, 3),
            0b1 => (144, 4),
            _ => fail!(~"unreachable"),
          },
        0b10 => (160 + ((rev_prefix & 0b10000) >> 0), 4),
        0b01 | 0b11 => (192 + (rev2((rev_prefix & 0b11000) >> 3) << 4), 4),
        _ => fail!(~"unreachable"),
      },
    _ => fail!(~"unreachable"),
  }
}

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
      (~error::ReferenceBeforeStart(3, 3, 2), &[0b0000_0000])
    );
  }
}

