use deflate::bit_reader::{BitReader};
use deflate::huffman_tree::{HuffmanTree};
use deflate::error::*;

pub fn read_code(in: &mut BitReader, tree: &HuffmanTree) -> u16 {
  let mut node = tree.root();
  while !tree.is_leaf(node) {
    node = if in.read_bit() == 0 {
        tree.zero_child(node)
      } else {
        tree.one_child(node)
      }
  }
  tree.leaf_value(node)
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

pub fn read_length(in: &mut BitReader, code: u16) -> uint {
  if code <= 264 {
    code as uint - 254
  } else if code < 285 {
    let rel = code as uint - 261;
    let extra = rel/4;
    let base: uint = (1<<(extra+2))+3+(rel%4)*(1<<extra);
    base + in.read_bits(extra) as uint
  } else if code == 285 {
    258
  } else {
    fail!(fmt!("read_length() got unknown length code %u", code as uint));
  }
}

pub fn read_dist(in: &mut BitReader, code: u16) -> uint {
  if code < 4 {
    code as uint + 1 
  } else if code <= 29 {
    let extra: uint = (code-2)/2 as uint; 
    let base = if code % 2 == 0 {
      1+1<<(extra+1)
    } else {
      1+3*(1<<extra)
    };
    base + in.read_bits(extra) as uint
  } else {
    // TODO: return an error?
    fail!(~"unexpected");
  }
}

pub fn decompress(in: &mut BitReader) -> Result<~[u8],~DeflateError> {
  let mut out: ~[u8] = ~[];

  loop {
    let bfinal = in.read_bit();
    let btype = in.read_bits(2);

    match btype {
      0b00 => {
        in.flush_byte();

        let lsb = in.read_byte();
        let msb = in.read_byte();
        let nlsb = in.read_byte();
        let nmsb = in.read_byte();

        let len: u16 = lsb as u16 | (msb as u16 << 8);
        let nlen: u16 = nlsb as u16 | (nmsb as u16 << 8);

        if len == !nlen {
          for (len as uint).times {
            out.push(in.read_byte());
          }
        } else {
          return Err(~LengthMismatchError(len as u16, nlen as u16))
        }
      },
      0b01 => {
        loop {
          let code = read_fix_code(in);

          if code < 256 {
            out.push(code as u8)
          } else if code == 256 {
            break
          } else {
            let len = read_length(in, code) as uint;

            let dist_code = in.read_rev_bits(5);
            let dist = read_dist(in, dist_code);

            for len.times {
              let byte = out[out.len() - dist];
              out.push(byte);
            }
          }
        }
      },
      _ => fail!(~"ouch")
    }

    if bfinal != 0 {
      break
    } else if in.eof() {
      return Err(~UnexpectedEOFError)
    }
  }
  
  Ok(out)
}

#[cfg(test)]

mod test {
  use deflate::decompress::{read_code, read_length, read_dist, read_fix_code,
  decompress};
  use deflate::huffman_tree::{from_bit_lengths};
  use deflate::bit_reader::{BitReader};
  use deflate::error::*;

#[test]
  fn test_read_code() {
    let (a,b,c,d,e,_f) = (0,1,2,3,4,5);
    let tree = from_bit_lengths(~[2,2,3,3,3,3]).unwrap();

    /*
      A   00
      B   01
      C   100
      D   101
      E   110
      F   111
    */

    let mut reader = BitReader::new(~[
      0b1000_0100, 0b0100_1110, 0b1000_0101 ]);

    assert_eq!(read_code(reader, tree), a);
    assert_eq!(read_code(reader, tree), c);
    assert_eq!(read_code(reader, tree), a);
    assert_eq!(read_code(reader, tree), d);
    assert_eq!(read_code(reader, tree), e);
    assert_eq!(read_code(reader, tree), b);

    assert_eq!(reader.read_bits(9), 0b1000_0101_0);
  }

#[test]
  fn test_decompress_non_compressed() {
    /* one block */
    let mut reader1 = BitReader::new(~[
        0b00000_001,
        0b00001010, 0b00000000,
        0b11110101, 0b11111111,
        10, 20, 30, 40, 50, 60, 70, 80, 90, 100
      ]);

    assert_eq!(decompress(reader1).unwrap(), ~[
        10, 20, 30, 40, 50, 60, 70, 80, 90, 100]);

    /* two blocks */
    let mut reader2 = BitReader::new(~[
        0b00000_000,
        0b0000_0110, 0b0000_0000,
        0b1111_1001, 0b1111_1111,
        11, 22, 33, 44, 55, 66,
        0b00000_001,
        0b0000_0100, 0b0000_0000,
        0b1111_1011, 0b1111_1111,
        77, 88, 99, 110
      ]);

    assert_eq!(decompress(reader2).unwrap(), ~[
      11, 22, 33, 44, 55, 66, 77, 88, 99, 110]);
  }

#[test]
  fn test_decompress_non_compressed_errors() {
    /* the length and the inverse don't match */
    let mut reader1 = BitReader::new(~[
      0b00000_001,
      0b0000_0101, 0b0000_0000,
      0b1100_0000, 0b1111_1111]);

    match decompress(reader1) {
      Err(~LengthMismatchError(0b0000_0000_0000_0101, 0b1111_1111_1100_0000)) =>
        { /* ok */ },
      Err(err) => fail!(fmt!("unexpected error %s", err.to_str())),
      _ => fail!(~"expected an error")
    }

    /* the stream is too short */
    let mut reader2 = BitReader::new(~[
        0b00000_000,
        0b00001010, 0b00000000,
        0b11110101, 0b11111111,
        10, 20, 30, 40 /* and 6 missing bytes */
      ]);

    match decompress(reader2) {
      Err(~UnexpectedEOFError) => { /* ok */ },
      Err(err) => fail!(fmt!("unexpected error %s", err.to_str())),
      _ => fail!(~"expected an error")
    }
  }

#[test]
  fn test_read_fix_code() {
    let mut reader = BitReader::new(~[
      0b0010_0111, 0b1111_1110, 0b1000_1001, 0b0111_0000,
      0b1000_1101, 0b0000_0001, 0b0110_1000, 0b0110_0011,
      0b0010_1001, 0b1101_0011, 0b1101_0110, 0b0000_1100,
      0b0000_0010]);

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

#[test]
  fn test_decompress_fixed() {
    /* literals only */
    let mut reader1 = BitReader::new(~[
      0b11100011, 0b00010010, 0b10010001, 0b00000011, 0b00000000]);

    assert_eq!(decompress(reader1).unwrap(), ~[10, 20, 30]);

    /* longer data */
    let mut reader2 = BitReader::new(~[
      0b11111011, 0b10110001, 0b01101010, 0b01101001,
      0b11111010, 0b10110111, 0b01111111, 0b01011110,
      0b11001101, 0b10111011, 0b10101011, 0b00000001]);

    assert_eq!(decompress(reader2).unwrap(), ~[
      248, 170, 165, 103, 246, 254, 74, 131, 187, 123]);

    /* simple length-distance pair */
    let mut reader3 = BitReader::new(~[
      0b10010011, 0b11010011, 0b00000010,
      0b00000010, 0b00001101, 0b00000000]);

    assert_eq!(decompress(reader3).unwrap(), ~[
      30, 42, 42, 42, 42, 40]);

    /* a longer length-distance pair */
    let mut reader4 = BitReader::new(~[
      0b11100011, 0b00010010, 0b01010011,
      0b11000100, 0b00001101, 0b10111001,
      0b11000100, 0b00010100, 0b00000001]);

    assert_eq!(decompress(reader4).unwrap(), ~[
      10, 22, 33, 22, 33, 22, 33, 22, 33, 22, 33, 22, 33, 22, 33, 22, 33, 22,
      33, 22, 33, 22, 33, 22, 33, 22, 33, 22, 33, 10, 22, 33]);

    /* long repetition and then long distance */
    let mut reader5 = BitReader::new(~[
      0b00010011, 0b10010001, 0b11010011, 0b00110000, 0b10110010,
      0b11100001, 0b00011010, 0b00000101, 0b00100011, 0b00001110,
      0b10001000, 0b10000000, 0b00100010, 0b00011110, 0b00000000]);

    let bytes5 = decompress(reader5).unwrap();

    assert_eq!(bytes5.slice(0, 5), &[20,30,40,50,60]);
    for uint::range(5, 505) |i| {
      assert_eq!(bytes5[i], 10);
    }
    assert_eq!(bytes5.slice(505, 510), &[20,30,40,50,60]);
  }

#[test]
  fn test_decompress_fixed_errors() {
    /* the distance is too long (points before the start of input) */
    // TODO
  }

#[test]
  fn test_read_length() {
    /* small and simple */
    let mut reader1 = BitReader::new(~[]);
    assert_eq!(read_length(reader1, 259), 5);
    assert_eq!(read_length(reader1, 263), 9);

    /* 3 extra bits */
    let mut reader2 = BitReader::new(~[0b110]);
    assert_eq!(read_length(reader2, 274), 43+6);

    /* 5 extra bits */
    let mut reader5 = BitReader::new(~[0b10011]);
    assert_eq!(read_length(reader5, 283), 195+19);
    
    /* special case - length 258 */
    let mut reader6 = BitReader::new(~[]);
    assert_eq!(read_length(reader6, 285), 258);
  }

#[test]
  fn test_read_dist() {
    /* small and simple */
    let mut reader1 = BitReader::new(~[]);
    assert_eq!(read_dist(reader1, 2), 3);
    assert_eq!(read_dist(reader1, 0), 1);

    /* 2 extra bits */
    let mut reader2 = BitReader::new(~[0b10]);
    assert_eq!(read_dist(reader2, 7), 13+2);

    /* 10 extra bits */
    let mut reader3 = BitReader::new(~[0b11100001, 0b11]);
    assert_eq!(read_dist(reader3, 23), 3073+993);
  }
}
