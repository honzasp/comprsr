use deflate::huffman_tree::{HuffmanTree};
use deflate::error::*;
use deflate::bit_reader::{BitReader};

pub fn read_huff_code(in: &mut BitReader, tree: &HuffmanTree) -> u16 {
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

pub fn read_length(in: &mut BitReader, code: u16) -> Result<uint,~DeflateError> {
  if code <= 264 {
    Ok(code as uint - 254)
  } else if code < 285 {
    let rel = code as uint - 261;
    let extra = rel/4;
    let base: uint = (1<<(extra+2))+3+(rel%4)*(1<<extra);
    Ok(base + in.read_bits(extra) as uint)
  } else if code == 285 {
    Ok(258)
  } else {
    Err(~BadLengthCode(code))
  }
}

pub fn read_dist(in: &mut BitReader, code: u16) -> Result<uint,~DeflateError> {
  if code < 4 {
    Ok(code as uint + 1)
  } else if code <= 29 {
    let extra: uint = (code-2)/2 as uint; 
    let base = if code % 2 == 0 {
      1+1<<(extra+1)
    } else {
      1+3*(1<<extra)
    };
    Ok(base + in.read_bits(extra) as uint)
  } else {
    Err(~BadDistCode(code))
  }
}

#[cfg(test)]
mod test {
  use deflate::decompress::compressed::{read_huff_code, read_length, read_dist};
  use deflate::bit_reader::{BitReader};
  use deflate::huffman_tree::{HuffmanTree};
  use deflate::error::*;

  #[test]
  fn test_read_huff_code() {
    let (a,b,c,d,e,_f) = (0,1,2,3,4,5);
    let tree = HuffmanTree::from_bit_lengths(~[2,2,3,3,3,3]).unwrap();

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

    assert_eq!(read_huff_code(reader, tree), a);
    assert_eq!(read_huff_code(reader, tree), c);
    assert_eq!(read_huff_code(reader, tree), a);
    assert_eq!(read_huff_code(reader, tree), d);
    assert_eq!(read_huff_code(reader, tree), e);
    assert_eq!(read_huff_code(reader, tree), b);

    assert_eq!(reader.read_bits(9), 0b1000_0101_0);
  }

  #[test]
  fn test_read_length() {
    /* small and simple */
    let mut reader1 = BitReader::new(~[]);
    assert_eq!(read_length(reader1, 259).unwrap(), 5);
    assert_eq!(read_length(reader1, 263).unwrap(), 9);

    /* 3 extra bits */
    let mut reader2 = BitReader::new(~[0b110]);
    assert_eq!(read_length(reader2, 274).unwrap(), 43+6);

    /* 5 extra bits */
    let mut reader5 = BitReader::new(~[0b10011]);
    assert_eq!(read_length(reader5, 283).unwrap(), 195+19);
    
    /* special case - length 258 */
    assert_eq!(read_length(reader1, 285).unwrap(), 258);

    /* wrong code */
    match read_length(reader1, 287) {
      Err(~BadLengthCode(287)) => { /* ok */ },
      Err(err) => fail!(fmt!("got error %s", err.to_str())),
      _ => fail!(~"expected error")
    }
  }

  #[test]
  fn test_read_dist() {
    /* small and simple */
    let mut reader1 = BitReader::new(~[]);
    assert_eq!(read_dist(reader1, 2).unwrap(), 3);
    assert_eq!(read_dist(reader1, 0).unwrap(), 1);

    /* 2 extra bits */
    let mut reader2 = BitReader::new(~[0b10]);
    assert_eq!(read_dist(reader2, 7).unwrap(), 13+2);

    /* 10 extra bits */
    let mut reader3 = BitReader::new(~[0b11100001, 0b11]);
    assert_eq!(read_dist(reader3, 23).unwrap(), 3073+993);

    /* wrong code */
    match read_dist(reader1, 30) {
      Err(~BadDistCode(30)) => { /* ok */ },
      Err(err) => fail!(fmt!("got error %s", err.to_str())),
      _ => fail!(~"expected error")
    }
  }
}
