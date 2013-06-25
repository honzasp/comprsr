use inflate::error;
use std::iterator::*;
use std::uint;
use std::vec;

pub struct Tree {
  priv nodes: ~[u16],
}

struct Node(u16);

impl Tree {
  pub fn new_empty() -> Tree {
    Tree { nodes: ~[] }
  }

  pub fn new_from_lens(bit_lens: &[u8]) -> Result<Tree, ~error::Error> {
    let mut bl_count: ~[int] = ~[0];
    let mut bl_syms: ~[~[u16]] = ~[~[]];
    let mut max_bl: uint = 0;

    for bit_lens.iter().enumerate().advance |(sym, &bl8)| {
      let bl = bl8 as uint;
      if bl > 0 {
        if bl > max_bl {
          vec::grow(&mut bl_count, bl - max_bl, &0);
          vec::grow(&mut bl_syms, bl - max_bl, &~[]);
          max_bl = bl;
        }
        bl_count[bl] += 1;
        bl_syms[bl].push(sym as u16);
      }
    }

    let mut nodes: ~[u16] = ~[0xffff];
    let mut front: ~[u16] = ~[0];
    let mut front_offset: uint = 0;

    for uint::range(1, max_bl+1) |level| {
      let mut new_front: ~[u16] = ~[];

      for uint::range(front_offset, front.len()) |i| {
        let front_n: u16 = front[i];
        let zero_n: u16 = nodes.len() as u16;
        let one_n: u16 = zero_n + 1;
        nodes[front_n] = zero_n << 1;

        vec::grow(&mut nodes, 2, &0xffff);
        new_front.push(zero_n);
        new_front.push(one_n);
      }

      front = new_front;
      front_offset = 0;

      if bl_syms[level].len() <= front.len() {
        for bl_syms[level].each |&sym| {
          let sym_n = front[front_offset];
          nodes[sym_n] = (sym << 1) | 0b1;

          front_offset = front_offset + 1;
        }
      } else {
        return Err(~error::TooManyHuffCodesError(level));
      }
    }

    Ok(Tree { nodes: nodes })
  }

  pub fn root(&self) -> Node {
    Node(0)
  }

  pub fn zero_child(&self, Node(n): Node) -> Node {
    Node(self.nodes[n] >> 1)
  }

  pub fn one_child(&self, Node(n): Node) -> Node {
    Node((self.nodes[n] >> 1) + 1)
  }

  pub fn is_leaf(&self, Node(n): Node) -> bool {
    self.nodes[n] & 0b1 != 0
  }

  pub fn leaf_value(&self, Node(n): Node) -> u16 {
    self.nodes[n] >> 1
  }

  pub fn is_defined(&self, Node(n): Node) -> bool {
    self.nodes[n] != 0xffff
  }
}

pub static undefined_leaf_value: u16 = 0xffff >> 1;

#[cfg(test)]
mod test {
  use inflate::huff;
  use inflate::error;

  #[test]
  fn test_tree_from_bit_lengths() {
    // example from RFC 1951 with zero-length codes 

    let (_a,_b,_c,_x,d,_e,f,_y,g,_h) = (0,1,2,3,4,5,6,7,8,9);
    let bit_lengths = ~[3,3,3,0,3,3,2,0,4,4];

    /*

    A 3  010
    B 3  011
    C 3  100
    D 3  101
    E 3  110
    F 2   00
    G 4 1110
    H 4 1111

               . 
           ---- ----
          /         \
         .           .
        / \         / \
       /   \       /   \
      F     .     .     .
           / \   / \   / \
          A   B C   D E   .
                         / \
                        G   H
     */

    let tree = ~huff::Tree::new_from_lens(bit_lengths).unwrap();

    let n00 = tree.zero_child(tree.zero_child(tree.root()));
    assert!(tree.is_leaf(n00));
    assert_eq!(tree.leaf_value(n00), f);

    let n10 = tree.zero_child(tree.one_child(tree.root()));
    assert!(!tree.is_leaf(n10));

    let n101 = tree.one_child(n10);
    assert!(tree.is_leaf(n101));
    assert_eq!(tree.leaf_value(n101), d);

    let n111 = tree.one_child(tree.one_child(tree.one_child(tree.root())));
    assert!(!tree.is_leaf(n111));

    let n1110 = tree.zero_child(n111);
    assert!(tree.is_leaf(n1110));
    assert_eq!(tree.leaf_value(n1110), g);
  }

  #[test]
  fn test_tree_from_invalid_bit_lengths() {
    /* too many 2's */
    assert_eq!(
      huff::Tree::new_from_lens([2,2,0,3,3,3,2,0]).get_err(),
      ~error::TooManyHuffCodesError(3)
    );
  }

  #[test]
  fn test_unsaturated_tree() {
    /* two 3-bit codes aren't defined:
          .        
         / \       
        /   \      
       .     .     
      / \   / \    
     2   3 /   \   
          .     .  
         / \   / \ 
        0   1 ?   ?
      */

    let tree = ~huff::Tree::new_from_lens([3,3,2,2]).unwrap();

    let n01 = tree.one_child(tree.zero_child(tree.root()));
    assert!(tree.is_defined(n01));

    let n110 = tree.zero_child(tree.one_child(tree.one_child(tree.root())));
    assert!(!tree.is_defined(n110));
    assert_eq!(tree.leaf_value(n110), huff::undefined_leaf_value);
  }
}
