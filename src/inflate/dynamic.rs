use inflate::bits;
use inflate::compressed;
use inflate::error;
use inflate::huff;

use std::vec;
use std::iterator::{IteratorUtil};

pub struct HeaderState {
  priv phase: HeaderPhase,
  priv meta_count: uint,
  priv meta_lens: ~[u8],
  priv meta_tree: ~huff::Tree,
  priv litlen_count: uint,
  priv dist_count: uint,
  priv code_count: uint, // litlen_count + dist_count
  priv code_lens: ~[u8],
}

enum HeaderPhase {
  NumbersPhase,
  MetaLensPhase,
  MetaPhase,
  CodeLensPhase,
  CodeLensRepeatPhase(u8, uint, uint),
  EndPhase,
}

#[deriving(Eq)]
pub enum MetaCode {
  LiteralMetaCode(u8),
  CopyMetaCode(uint, uint),
  ZeroesMetaCode(uint, uint),
}

pub fn decode_meta(code: uint) -> Result<MetaCode, ~error::Error> {
  match code {
    x if x <= 15 => Ok(LiteralMetaCode(x as u8)),
    16 => Ok(CopyMetaCode(3, 2)),
    17 => Ok(ZeroesMetaCode(3, 3)),
    18 => Ok(ZeroesMetaCode(11, 7)),
    y  => Err(~error::BadMetaCode(y)),
  } 
}

static meta_len_order: [u8, ..19] = 
  [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];

impl HeaderState {
  pub fn new() -> HeaderState {
    HeaderState {
      phase: NumbersPhase,
      meta_count: 0, meta_lens: ~[], meta_tree: ~huff::Tree::new_empty(),
      litlen_count: 0, dist_count: 0, code_count: 0, code_lens: ~[],
    }
  }

  pub fn input(&mut self, bit_reader: &mut bits::BitReader)
    -> Option<Result<~BlockState,~error::Error>>
  {
    loop {
      self.phase = match self.phase {
        NumbersPhase => {
          if bit_reader.has_bits(14) {
            let hlit = bit_reader.read_bits8(5);
            let hdist = bit_reader.read_bits8(5);
            let hclen = bit_reader.read_bits8(4);

            self.meta_count = hclen as uint + 4;
            self.litlen_count = hlit as uint + 257;
            self.dist_count = hdist as uint + 1;
            self.code_count = self.litlen_count + self.dist_count;

            vec::reserve(&mut self.meta_lens, self.meta_count);
            vec::reserve(&mut self.code_lens, self.code_count);
            MetaLensPhase
          } else {
            return None
          }
        },
        MetaLensPhase => {
          while self.meta_lens.len() < self.meta_count {
            if bit_reader.has_bits(3) {
              let len = bit_reader.read_bits8(3);
              self.meta_lens.push(len);
            } else {
              return None
            }
          }
          MetaPhase
        },
        MetaPhase => {
          let mut meta_code_lens = ~[0, ..19];
          for self.meta_lens.iter().zip(meta_len_order.iter()).advance 
            |(&len, &code)|
          {
            meta_code_lens[code] = len;
          }

          self.meta_tree = match huff::Tree::new_from_lens(meta_code_lens) {
              Ok(tree) => ~tree,
              Err(err) => return Some(Err(err)),
            };

          CodeLensPhase
        },
        CodeLensPhase => 
          if self.code_lens.len() < self.code_count {
            match read_huff_code(bit_reader, self.meta_tree) {
              Some(code) => match decode_meta(code) {
                Ok(LiteralMetaCode(len)) => {
                  self.code_lens.push(len);
                  CodeLensPhase
                },
                Ok(CopyMetaCode(count_base, count_extra_bits)) =>
                  match self.code_lens.last_opt() {
                    Some(&last_code) =>
                      CodeLensRepeatPhase(last_code, count_base, count_extra_bits),
                    None =>
                      return Some(Err(~error::MetaCopyAtStart)),
                  },
                Ok(ZeroesMetaCode(count_base, count_extra_bits)) =>
                  CodeLensRepeatPhase(0, count_base, count_extra_bits),
                Err(err) => return Some(Err(err)),
              },
              None => return None,
            }
          } else {
            EndPhase
          },
        CodeLensRepeatPhase(len_to_repeat, count_base, count_extra_bits) =>
          if bit_reader.has_bits(count_extra_bits) {
            let extra = bit_reader.read_bits8(count_extra_bits);
            let repeat_count = count_base + extra as uint;

            if self.code_lens.len() + repeat_count <= self.code_count {
              for repeat_count.times() {
                self.code_lens.push(len_to_repeat);
              }
              CodeLensPhase
            } else {
              return Some(Err(~error::MetaRepeatTooLong(
                  len_to_repeat, repeat_count, self.code_count - self.code_lens.len()
                )))
            }
          } else {
            return None
          },
        EndPhase => {
          let litlen_slice = self.code_lens.slice(0, self.litlen_count);
          let dist_slice = self.code_lens.slice(self.litlen_count,
              self.litlen_count + self.dist_count);

          let litlen_tree = match huff::Tree::new_from_lens(litlen_slice) {
              Ok(tree) => ~tree,
              Err(err) => return Some(Err(err)),
            };
          let dist_tree = match huff::Tree::new_from_lens(dist_slice) {
              Ok(tree) => ~tree,
              Err(err) => return Some(Err(err)),
            };

          let block_state = ~compressed::BlockState {
              phase: compressed::LitlenPhase,
              extra: BlockExtra {
                litlen_tree: litlen_tree,
                dist_tree: dist_tree,
              },
            };

          return Some(Ok(block_state));
        }
      }
    }
  }
}

pub type BlockState = compressed::BlockState<BlockExtra>;
pub struct BlockExtra {
  priv litlen_tree: ~huff::Tree,
  priv dist_tree: ~huff::Tree,
}

impl compressed::BlockExtra for BlockExtra {
  fn read_litlen_code(&self, bit_reader: &mut bits::BitReader)
    -> Option<uint>
  {
    read_huff_code(bit_reader, self.litlen_tree)
  }

  fn read_dist_code(&self, bit_reader: &mut bits::BitReader) 
    -> Option<uint>
  {
    read_huff_code(bit_reader, self.dist_tree)
  }
}

fn read_huff_code(bit_reader: &mut bits::BitReader, huff_tree: &huff::Tree)
  -> Option<uint>
{
  let mut read_data = 0;
  let mut read_bits = 0;
  let mut node = huff_tree.root();

  while !huff_tree.is_leaf(node) {
    if bit_reader.has_bits(1) {
      let bit = bit_reader.read_bits8(1);
      read_data = read_data | (bit << read_bits);
      read_bits = read_bits + 1;

      node = if bit == 0 {
          huff_tree.zero_child(node)
        } else {
          huff_tree.one_child(node)
        };
    } else {
      bit_reader.unread_bits8(read_bits, read_data);
      return None;
    }
  }

  Some(huff_tree.leaf_value(node) as uint)
}

#[cfg(test)]
mod test {
  use inflate::test_helpers::*;

  #[test]
  fn test_inflate_dynamic() {
    { // short and hand-made
      assert_eq!(inflate_ok(&[
          0b00001101, 0b11000101, 0b10110001, 0b00000001, 0b00000000,
          0b00000000, 0b00001000, 0b11000010, 0b10110000, 0b01010010,
          0b11111000, 0b11111111, 0b01100110, 0b11001101, 0b10010010,
          0b10101100, 0b00000001, 0b11011100, 0b10001100, 0b01100010,
          0b11111101, 0b01001001, 0b00001111
        ]),
        ~[1,4,3,1,0,0,0,2,4,4,2,1,2,2,0,2,3,2,2,1,2,0,1,3]
      );
    }

    { // longer output of zlib
      assert_eq!(inflate_ok(&[
          0b00001101, 0b11001000, 0b10110001, 0b00000001, 0b00000000,
          0b00100000, 0b00001100, 0b00000010, 0b00110000, 0b00101100,
          0b01010010, 0b11100000, 0b11111111, 0b10000111, 0b00110101,
          0b01100011, 0b01101110, 0b00101000, 0b11110001, 0b11001100,
          0b00110110, 0b10101000, 0b01010001, 0b11001000, 0b11110001,
          0b11111010, 0b00011111, 0b11010101, 0b01111001,
        ]),
        ~[4, 8, 3, 5, 5, 3, 1, 2, 6, 9
        , 8, 0, 9, 7, 0, 9, 0, 5, 7, 8
        , 7, 6, 7, 5, 5, 3, 3, 5, 9, 2]
      );
    }

    { // even longer output of zlib
      assert_eq!(inflate_ok(&[
         0b00001101, 0b11001000, 0b00110001, 0b00010001, 0b00000000,
         0b00000000, 0b00001100, 0b00000010, 0b00110001, 0b01000100,
         0b01110000, 0b10001100, 0b11101000, 0b01111100, 0b11110111,
         0b10110100, 0b00011001, 0b01010011, 0b01001000, 0b00100111,
         0b10001111, 0b01111001, 0b00000011, 0b00100101, 0b00111111,
         0b00110110, 0b01010110, 0b11001010, 0b00000001 
        ]),
        ~[30, 120, 120, 22, 30, 255, 0, 20, 255,
          120, 255, 20, 255, 255, 120, 120, 0, 22,
          22, 120, 120, 22, 20, 20, 120, 20, 0, 22,
          30, 120]
      );
    }
  }
}

