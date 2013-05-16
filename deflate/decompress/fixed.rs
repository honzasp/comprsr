use deflate::error::*;
use deflate::bit_reader::{BitReader};
use deflate::output::{Output};

use deflate::decompress::compressed::{compressed_block};

pub fn fixed_compressed_block(in: &mut BitReader, out: &mut Output)
-> Option<~DeflateError>
{
  compressed_block(in, out,
      |in| read_fix_code(in),
      |in| in.read_rev_bits(5)
    )
}

pub fn read_fix_code(in: &mut BitReader) -> u16 {
  if in.read_bit() == 0 { // 0
    if in.read_bit() == 0 { // 00
      if in.read_bit() == 0 { // 000
        in.read_rev_bits(4) + 256
      } else { // 001
        if in.read_bit() == 0 { // 0010
          in.read_rev_bits(3) + 272
        } else { // 0011
          in.read_rev_bits(4) + 0
        }
      }
    } else { // 01
      in.read_rev_bits(6) + 16
    }
  } else { // 1
    if in.read_bit() == 0 { // 10
      in.read_rev_bits(6) + 80
    } else { // 11
      if in.read_bit() == 0 { // 110
        if in.read_bit() == 0 { // 1100
          if in.read_bit() == 0 { // 11000 {
            in.read_rev_bits(3) + 280
          } else { // 11001
            in.read_rev_bits(4) + 144
          }
        } else { // 1101
          in.read_rev_bits(5) + 160
        }
      } else { // 111
        in.read_rev_bits(6) + 192
      }
    }
  }
}

#[cfg(test)]
mod test {
  use deflate::decompress::fixed::{read_fix_code};
  use deflate::bit_reader::{read_bytes};

  #[test]
  fn test_read_fix_code() {
    do read_bytes(&[
      0b0010_0111, 0b1111_1110, 0b1000_1001, 0b0111_0000,
      0b1000_1101, 0b0000_0001, 0b0110_1000, 0b0110_0011,
      0b0010_1001, 0b1101_0011, 0b1101_0110, 0b0000_1100,
      0b0000_0010]) |mut reader| {
      assert_eq!(read_fix_code(reader), 200);
      assert_eq!(read_fix_code(reader), 254);
      assert_eq!(read_fix_code(reader), 20);
      assert_eq!(read_fix_code(reader), 10);
      assert_eq!(read_fix_code(reader), 286);
      assert_eq!(read_fix_code(reader), 256);
      assert_eq!(read_fix_code(reader), 278);
      assert_eq!(read_fix_code(reader), 286);
      assert_eq!(read_fix_code(reader), 100);
      assert_eq!(read_fix_code(reader), 150);
      assert_eq!(read_fix_code(reader), 172);
      assert_eq!(read_fix_code(reader), 281);
    }
  }
}
