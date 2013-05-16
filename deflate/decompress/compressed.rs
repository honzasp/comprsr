use deflate::huffman_tree::{HuffmanTree};
use deflate::error::*;
use deflate::bit_reader::{BitReader};
use deflate::output::{Output};

pub fn compressed_block(
  in: &mut BitReader,
  out: &mut Output,
  read_litlen_code: &fn(&mut BitReader) -> u16,
  read_dist_code: &fn(&mut BitReader) -> u16
) -> Option<~DeflateError>
{
  loop {
    let litlen = read_litlen_code(in);

    if litlen < 256 {
      out.write(litlen as u8)
    } else if litlen == 256 {
      break
    } else {
      let len = match read_length(in, litlen) {
          Ok(len) => len,
          Err(err) => return Some(err)
        };

      let dist_code = read_dist_code(in);
      let dist = match read_dist(in, dist_code) {
          Ok(dist) => dist,
          Err(err) => return Some(err)
        };

      if out.window_len() >= dist {
        out.repeat(len, dist);
      } else {
        return Some(~DistanceTooLong(out.len(), dist))
      }
    }
  }

  None
}

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

pub fn read_length(in: &mut BitReader, code: u16) 
  -> Result<uint,~DeflateError> 
{
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
      1+(1<<(extra+1))
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
  use deflate::bit_reader::{read_bytes};
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

    do read_bytes(&[0b1000_0100, 0b0100_1110, 0b1000_0101 ]) |mut reader| {
      assert_eq!(read_huff_code(reader, tree), a);
      assert_eq!(read_huff_code(reader, tree), c);
      assert_eq!(read_huff_code(reader, tree), a);
      assert_eq!(read_huff_code(reader, tree), d);
      assert_eq!(read_huff_code(reader, tree), e);
      assert_eq!(read_huff_code(reader, tree), b);

      assert_eq!(reader.read_bits(9), 0b1000_0101_0);
    }
  }

  #[test]
  fn test_read_length() {
    /* small and simple */
    do read_bytes(&[]) |mut reader1| {
      assert_eq!(read_length(reader1, 259).unwrap(), 5);
      assert_eq!(read_length(reader1, 263).unwrap(), 9);
    }

    /* 3 extra bits */
    do read_bytes(&[0b110]) |mut reader2| {
      assert_eq!(read_length(reader2, 274).unwrap(), 43+6);
    }

    /* 5 extra bits */
    do read_bytes(&[0b10011]) |mut reader5| {
      assert_eq!(read_length(reader5, 283).unwrap(), 195+19);
    }

    /* special case - length 258 (the bits should be ignored) */
    do read_bytes(&[0b10101010]) |mut reader| {
      assert_eq!(read_length(reader, 285).unwrap(), 258);
    }

    /* wrong code */
    do read_bytes(&[]) |mut reader| {
      match read_length(reader, 287) {
        Err(~BadLengthCode(287)) => { /* ok */ },
        Err(err) => fail!(fmt!("got error %s", err.to_str())),
        _ => fail!(~"expected error")
      }
    }
  }

  #[test]
  fn test_read_dist() {
    /* small and simple */
    do read_bytes(&[]) |mut reader0| { 
      assert_eq!(read_dist(reader0, 2).unwrap(), 3);
      assert_eq!(read_dist(reader0, 0).unwrap(), 1);
    }

    /* 1 extra bit */
    do read_bytes(&[0b01]) |mut reader1| { 
      assert_eq!(read_dist(reader1, 4).unwrap(), 5+1);
      assert_eq!(read_dist(reader1, 5).unwrap(), 7);
    }

    /* 2 extra bits */
    do read_bytes(&[0b01_10]) |mut reader2| { 
      assert_eq!(read_dist(reader2, 7).unwrap(), 13+2);
      assert_eq!(read_dist(reader2, 6).unwrap(), 10);
    }

    /* 3 extra bits */
    do read_bytes(&[0b000_100]) |mut reader3| { 
      assert_eq!(read_dist(reader3, 8).unwrap(), 17+4);
      assert_eq!(read_dist(reader3, 9).unwrap(), 25);
    }

    /* 5 extra bits */
    do read_bytes(&[0b001_10010, 0b00]) |mut reader5| { 
      assert_eq!(read_dist(reader5, 13).unwrap(), 97+18);
      assert_eq!(read_dist(reader5, 12).unwrap(), 65+1);
    }

    /* 7 extra bits */
    do read_bytes(&[0b0_0000010, 0b000011]) |mut reader7| { 
      assert_eq!(read_dist(reader7, 17).unwrap(), 385+2);
      assert_eq!(read_dist(reader7, 16).unwrap(), 257+6);
    }

    /* 10 extra bits */
    do read_bytes(&[0b11100001, 0b11]) |mut reader10| { 
      assert_eq!(read_dist(reader10, 23).unwrap(), 3073+993);
    }

    /* wrong code */
    do read_bytes(&[]) |mut reader| {
      match read_dist(reader, 30) {
        Err(~BadDistCode(30)) => { /* ok */ },
        Err(err) => fail!(fmt!("got error %s", err.to_str())),
        _ => fail!(~"expected error")
      }
    }
  }
}
