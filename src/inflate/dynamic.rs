use inflate::bits;
use inflate::compressed::*;
use inflate::error;
use inflate::huff;
use inflate::out;

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
                Ok(meta_code) => match meta_code {
                  LiteralMetaCode(len) => {
                    self.code_lens.push(len);
                    CodeLensPhase
                  },
                  CopyMetaCode(count_base, count_extra_bits) =>
                    match self.code_lens.head_opt() {
                      Some(&last_code) =>
                        CodeLensRepeatPhase(last_code, count_base, count_extra_bits),
                      None =>
                        return Some(Err(~error::MetaCopyAtStart)),
                    },
                  ZeroesMetaCode(count_base, count_extra_bits) =>
                    CodeLensRepeatPhase(0, count_base, count_extra_bits),
                },
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

          let block_state = ~BlockState {
              phase: LitlenPhase,
              litlen_tree: litlen_tree,
              dist_tree: dist_tree,
            };

          return Some(Ok(block_state));
        }
      }
    }
  }
}

pub struct BlockState {
  priv phase: BlockPhase,
  priv litlen_tree: ~huff::Tree,
  priv dist_tree: ~huff::Tree,
}

impl BlockState {
  pub fn input(&mut self, bit_reader: &mut bits::BitReader, out: &mut out::Output)
    -> Option<Result<(),~error::Error>>
  {
    // TODO: create a generic dynamic/fixed input method
    loop {
      self.phase = match self.phase {
        LitlenPhase => 
          match read_huff_code(bit_reader, self.litlen_tree) {
            Some(code) => match decode_litlen(code) {
              Ok(litlen) => match litlen {
                LiteralCode(byte) => {
                  out.send_literal(byte);
                  LitlenPhase
                },
                LengthCode(len, 0) =>
                  DistPhase(len),
                LengthCode(len_base, len_extra_bits) =>
                  LenExtraPhase(len_base, len_extra_bits),
                BlockEndCode =>
                  return Some(Ok(())),
              },
              Err(err) =>
                return Some(Err(err)),
            },
            None => 
              return None
          },
        LenExtraPhase(len_base, len_extra_bits) => 
          if bit_reader.has_bits(len_extra_bits) {
            let extra = bit_reader.read_bits8(len_extra_bits);
            DistPhase(len_base + extra as uint)
          } else {
            return None;
          },
        DistPhase(len) =>
          match read_huff_code(bit_reader, self.dist_tree) {
            Some(dist_code) => match decode_dist(dist_code) {
              Ok((dist_base, dist_extra_bits)) =>
                DistExtraPhase(len, dist_base, dist_extra_bits),
              Err(err) =>
                return Some(Err(err)),
            },
            None => return None,
          },
        DistExtraPhase(len, dist_base, dist_extra_bits) => 
          if bit_reader.has_bits(dist_extra_bits) {
            let dist_extra = bit_reader.read_bits16(dist_extra_bits);
            let dist = dist_base + dist_extra as uint;
            match out.back_reference(dist, len) {
              Ok(()) => LitlenPhase,
              Err(err) => return Some(Err(err)),
            }
          } else {
            return None;
          },
      }
    }
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
}

