use deflate::huffman_tree::{HuffmanTree};
use deflate::error::*;
use deflate::bit_reader::{BitReader};
use deflate::output::{Output};

use deflate::decompress::compressed::{read_huff_code, compressed_block};

pub fn dynamic_compressed_block(in: &mut BitReader, out: &mut Output)
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

  compressed_block(in, out,
      |in| read_huff_code(in, litlen_tree),
      |in| read_huff_code(in, dist_tree)
    )
}

pub fn read_meta_tree(in: &mut BitReader, count: uint) 
  -> Result<~HuffmanTree,~DeflateError> 
{
  let metacode_order: &[uint] = &[
    16,17,18,0,8,7,9,6,10,5,11,4,12,3,13,2,14,1,15];

  let mut meta_bitlens: ~[u8] = ~[];
  vec::grow(&mut meta_bitlens, 19, &0);

  for uint::range(0, count) |i| {
    meta_bitlens[metacode_order[i]] = in.read_bits(3) as u8;
  }

  HuffmanTree::from_bit_lengths(meta_bitlens)
}

pub fn read_huff_tree(
  in: &mut BitReader,
  meta_tree: &HuffmanTree,
  symbol_count: uint)
-> Result<~HuffmanTree,~DeflateError>
{
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
          16 => if symbol > 0 {
              (3 + in.read_bits(2) as uint, bitlens[symbol - 1])
            } else { return Err(~MetaRepeatAtStart) },
          17 => (3 + in.read_bits(3) as uint, 0),
          18 => (11 + in.read_bits(7) as uint, 0),
          _  => if code == HuffmanTree::undefined() {
              return Err(~MetaUndefinedCode)
            } else {
              fail!(fmt!("read_huff_tree() read invalid meta code %u", code as uint))
            }
        };

      if symbol + repeat <= bitlens.len() {
        for uint::range(symbol, symbol + repeat) |i| {
          bitlens[i] = value;
        }

        symbol = symbol + repeat;
      } else {
        return Err(~MetaRepeatTooLong(repeat, bitlens.len() - symbol));
      }
    }
  }

  HuffmanTree::from_bit_lengths(bitlens)
}

#[cfg(test)]
mod test {
  use deflate::decompress::dynamic::{read_huff_tree, read_meta_tree};
  use deflate::bit_reader::{read_bytes};
  use deflate::huffman_tree::{HuffmanTree};
  use deflate::error::*;

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

    do read_bytes(&[
      0b0001_0000, 0b1000_0100, 0b0000_1001,
      0b0000_0000, 0b0011_0000])
    |mut reader1| {
      let t1 = read_meta_tree(reader1, 13).unwrap();
      let n00 = t1.zero_child(t1.zero_child(t1.root()));
      let n1 = t1.one_child(t1.root());

      assert_eq!(t1.leaf_value(n00), 0);
      assert_eq!(t1.leaf_value(t1.zero_child(n1)), 17);
      assert_eq!(t1.leaf_value(t1.one_child(t1.one_child(n1))), 12);
    }
  }

  #[test]
  fn test_read_huff_tree() {
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

    let meta_tree = HuffmanTree::from_bit_lengths(&[
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

    do read_bytes(&[
      0b10001000, 0b00101000, 0b10110010, 0b10101000,
      0b10000000, 0b11111111, 0b11111111, 0b11101011,
      0b00100100, 0b0011])
    |mut reader| {
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
  }

  #[test]
  fn test_read_huff_tree_errors() {
    /* the meta alphabet:
      0   00
      16  01
      3   100
      15  101
      18  110
      ??  111 (undefined)
    */
    let meta_tree = HuffmanTree::from_bit_lengths(~[
      2,0,0,3,0,0,0,0,0,0,0,0,0,0,0,3,2,0,3]).unwrap();

    /* error 1: the first metacode is 16 (copying the previous code) */
    do read_bytes(&[0b10]) |mut reader1| {
      match read_huff_tree(reader1, meta_tree, 260) {
        Err(~MetaRepeatAtStart) => { },
        Err(err) => fail!(fmt!("unexpected %s", err.to_str())),
        Ok(_) => fail!(~"expected an error")
      }
    }

    /* error 2: repeat too long */
    do read_bytes(&[0b1111_1011, 0b1000_1111, 0b0000_1111]) |mut reader2| {
      match read_huff_tree(reader2, meta_tree, 263) {
        Err(~MetaRepeatTooLong(135, 125)) => { },
        Err(err) => fail!(fmt!("unexpected %s", err.to_str())),
        Ok(_) => fail!(~"expected an error")
      }
    }
    
    /* error 3: undefined meta code */
    do read_bytes(&[0b111001]) |mut reader3| {
      match read_huff_tree(reader3, meta_tree, 261) {
        Err(~MetaUndefinedCode) => { },
        Err(err) => fail!(fmt!("unexpected %s", err.to_str())),
        Ok(_) => fail!(~"expected an error")
      }
    }
  }
}
