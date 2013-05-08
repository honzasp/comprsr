use deflate::bit_reader::{BitReader};
use deflate::huffman_tree::{HuffmanTree};
use deflate::error::*;

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

fn non_compressed_block(in: &mut BitReader, out: &mut ~[u8])
  -> Option<~DeflateError> 
{
  in.flush_byte();

  let lsb = in.read_byte();
  let msb = in.read_byte();
  let nlsb = in.read_byte();
  let nmsb = in.read_byte();

  let len: u16 = lsb as u16 | (msb as u16 << 8);
  let nlen: u16 = nlsb as u16 | (nmsb as u16 << 8);

  if len == !nlen {
    for (len as uint).times {
      vec::push(out, in.read_byte());
    }
  } else {
    return Some(~LengthMismatchError(len as u16, nlen as u16))
  }

  None
}

fn fixed_compressed_block(in: &mut BitReader, out: &mut ~[u8])
  -> Option<~DeflateError>
{
  loop {
    let litlen = read_fix_code(in);

    if litlen < 256 {
      out.push(litlen as u8)
    } else if litlen == 256 {
      break
    } else {
      let len = match read_length(in, litlen) {
          Ok(len) => len,
          Err(err) => return Some(err)
        };

      let dist_code = in.read_rev_bits(5);
      let dist = match read_dist(in, dist_code) {
          Ok(dist) => dist,
          Err(err) => return Some(err)
        };

      if out.len() >= dist {
        for len.times {
          let byte = out[out.len() - dist];
          out.push(byte);
        }
      } else {
        return Some(~DistanceTooLong(out.len(), dist))
      }
    }
  }

  None
}

fn dynamic_compressed_block(in: &mut BitReader, out: &mut ~[u8])
  -> Option<~DeflateError>
{
  let hlit = in.read_bits(5);
  let hdist = in.read_bits(5);
  let hclen = in.read_bits(4);

  let meta_tree = match read_meta_tree(in, hclen as uint + 4) {
      Ok(meta_tree) => meta_tree,
      Err(err) => return Some(err)
    };

  let litlen_tree = match read_huff_tree(in, meta_tree, hlit as uint + 257) {
      Ok(litlen_tree) => litlen_tree,
      Err(err) => return Some(err)
    };

  let dist_tree = match read_huff_tree(in, meta_tree, hdist as uint + 1) {
      Ok(dist_tree) => dist_tree,
      Err(err) => return Some(err)
    };

  loop {
    let litlen = read_huff_code(in, litlen_tree);

    // TODO: lot of duplication!

    if litlen < 256 {
      out.push(litlen as u8)
    } else if litlen == 256 {
      break
    } else {
      let len = match read_length(in, litlen) {
          Ok(len) => len,
          Err(err) => return Some(err)
        };

      let dist_code = read_huff_code(in, dist_tree);
      let dist = match read_dist(in, dist_code) {
          Ok(dist) => dist,
          Err(err) => return Some(err)
        };

      if out.len() >= dist {
        for len.times {
          let byte = out[out.len() - dist];
          out.push(byte);
        }
      } else {
        return Some(~DistanceTooLong(out.len(), dist))
      }
    }
  }

  None
}

pub fn read_meta_tree(in: &mut BitReader, count: uint) 
  -> Result<~HuffmanTree,~DeflateError> 
{
  use deflate::huffman_tree::from_bit_lengths;

  let metacode_order: &[uint] = &[
    16,17,18,0,8,7,9,6,10,5,11,4,12,3,13,2,14,1,15];

  let mut meta_bitlens: ~[u8] = ~[];
  vec::grow(&mut meta_bitlens, 19, &0);

  for uint::range(0, count) |i| {
    meta_bitlens[metacode_order[i]] = in.read_bits(3) as u8;
  }

  from_bit_lengths(meta_bitlens)
}

pub fn read_huff_tree(in: &mut BitReader, meta_tree: &HuffmanTree,
  symbol_count: uint) -> Result<~HuffmanTree,~DeflateError>
{
  use deflate::huffman_tree::from_bit_lengths;

  let mut bitlens: ~[u8] = ~[];
  vec::grow(&mut bitlens, symbol_count, &0);

  let mut symbol = 0;
  while symbol < symbol_count {
    let code = read_huff_code(in, meta_tree);

    if code <= 15 {
      bitlens[symbol] = code as u8;
      symbol = symbol + 1;
    } else {
      let (repeat,value) = match code {
          // TODO: what if it's the first?
          // TODO: what if the repeat is too long? (over bitlens[])
          16 => (3 + in.read_bits(2) as uint, bitlens[symbol - 1]),
          17 => (3 + in.read_bits(3) as uint, 0),
          18 => (11 + in.read_bits(7) as uint, 0),
          _  => fail!(fmt!("read_litlen_tree() read invalid meta code %u",
            code as uint))
        };

      for uint::range(symbol, symbol + repeat) |i| {
        bitlens[i] = value;
      }

      symbol = symbol + repeat;
    }
  }

  from_bit_lengths(bitlens)
}

pub fn decompress(in: &mut BitReader) -> Result<~[u8],~DeflateError> {
  let mut out: ~[u8] = ~[];

  loop {
    let bfinal = in.read_bit();
    let btype = in.read_bits(2);

    match btype {
      0b00 => match non_compressed_block(in, &mut out) {
          Some(err) => return Err(err), _ => { }
        },
      0b01 => match fixed_compressed_block(in, &mut out) {
          Some(err) => return Err(err), _ => { }
        },
      0b10 => match dynamic_compressed_block(in, &mut out) {
          Some(err) => return Err(err), _ => { }
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
  use deflate::decompress::{read_huff_code, read_meta_tree, read_huff_tree,
    read_length, read_dist, read_fix_code, decompress};
  use deflate::huffman_tree::{from_bit_lengths};
  use deflate::bit_reader::{BitReader};
  use deflate::error::*;

#[test]
  fn test_read_huff_code() {
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

    assert_eq!(read_huff_code(reader, tree), a);
    assert_eq!(read_huff_code(reader, tree), c);
    assert_eq!(read_huff_code(reader, tree), a);
    assert_eq!(read_huff_code(reader, tree), d);
    assert_eq!(read_huff_code(reader, tree), e);
    assert_eq!(read_huff_code(reader, tree), b);

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
    let mut reader1 = BitReader::new(~[
      0b1110_0011, 0b0001_0010, 0b0000_0011,
      0b0001_0010, 0b0000_0000]);

    match decompress(reader1) {
      Err(~DistanceTooLong(2, 8)) => { /* ok */ },
      Err(err) => fail!(fmt!("got error %s", err.to_str())),
      _ => fail!(~"expected error")
    }
  }

#[test]
  fn test_decompress_dynamic() {
    let mut reader1 = BitReader::new(~[
      0b00001101, 0b11000101, 0b10110001, 0b00000001, 0b00000000,
      0b00000000, 0b00001000, 0b11000010, 0b10110000, 0b01010010,
      0b11111000, 0b11111111, 0b01100110, 0b11001101, 0b10010010,
      0b10101100, 0b00000001, 0b11011100, 0b10001100, 0b01100010,
      0b11111101, 0b01001001, 0b00001111]);

    assert_eq!(decompress(reader1).unwrap(), ~[
      1,4,3,1,0,0,0,2,4,4,2,1,2,2,0,2,3,2,2,1,2,0,1,3]);
  }

#[test]
  fn test_read_meta_tree() {
    /*
        .  
       / \--
      /     \
     .       .
    / \     / \
   0   9  17   .
              / \
             7   12

    code 16 17 18 0 8 7 9 6 10 5 11 4 12 3 13 2 14 1 15
    len   0  2  0 2 0 3 2 0  0 0  0 0  3 ...
    */

    let mut reader1 = BitReader::new(~[
      0b0001_0000, 0b1000_0100, 0b0000_1001,
      0b0000_0000, 0b0011_0000]);

    let t1 = read_meta_tree(reader1, 13).unwrap();
    let n00 = t1.zero_child(t1.zero_child(t1.root()));
    let n1 = t1.one_child(t1.root());

    assert_eq!(t1.leaf_value(n00), 0);
    assert_eq!(t1.leaf_value(t1.zero_child(n1)), 17);
    assert_eq!(t1.leaf_value(t1.one_child(t1.one_child(n1))), 12);
  }

#[test]
  fn test_read_huff_tree() {
    use deflate::huffman_tree::from_bit_lengths;

    /* meta ("code length") alphabet:
      4   00
      5   01
      0   100
      3   101
      6   1100
      16  1101
      17  1110
      18  1111
    */

    let meta_tree = from_bit_lengths(&[
        3,0,0,3,2,2,4,0,0,0, 0,0,0,0,0,0,4,4,4]).unwrap();

    /* encoded tree;
      lit bits  code
      --------------
      0    4    0000
      1    5    10110
      2    4    0001
      3    5    10111
      4    4    0010
      5    5    11000
      6    5    11001
      7    4    0011
      8    5    11010
      9    4    0100
      10   4    0101
      11   4    0110
      12   4    0111
      13   5    11011
      14   5    11100
      15   5    11101
      16   4    1000
      17   4    1001
      18   4    1010
      19   5    11110
      ...  0
      256  6    111110
      257  0
      258  0
      259  6    111111
      */

    let mut reader = BitReader::new(~[
      0b10001000, 0b00101000, 0b10110010, 0b10101000,
      0b10000000, 0b11111111, 0b11111111, 0b11101011,
      0b00100100, 0b0011]);

    let tree = read_huff_tree(reader, meta_tree, 260).unwrap();

    let r = tree.root();
    let n0 = |n| tree.zero_child(n);
    let n1 = |n| tree.one_child(n);
    let val = |n| tree.leaf_value(n);
    
    assert_eq!(val(n0(n0(n0(n0(r))))), 0);
    assert_eq!(val(n1(n0(n0(n0(r))))), 2);
    assert_eq!(val(n0(n1(n0(n1(r))))), 18);

    assert_eq!(val(n0(n0(n0(n1(n1(r)))))), 5);
    assert_eq!(val(n0(n1(n0(n1(n1(r)))))), 8);

    assert_eq!(val(n0(n1(n1(n1(n1(n1(r))))))), 256);
    assert_eq!(val(n1(n1(n1(n1(n1(n1(r))))))), 259);
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
