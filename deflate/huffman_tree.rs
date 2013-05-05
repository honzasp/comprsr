use deflate::error::*;

pub struct HuffmanTree {
  nodes: ~[u16]
}
type HuffmanNode = u16;

impl HuffmanTree {
  pub fn root(&self) -> HuffmanNode {
    0
  }

  pub fn is_leaf(&self, n: HuffmanNode) -> bool {
    self.nodes[n] == 0
  }

  pub fn leaf_value(&self, n: HuffmanNode) -> u16 {
    self.nodes[n+1]
  }

  pub fn zero_child(&self, n: HuffmanNode) -> HuffmanNode {
    self.nodes[n]
  }

  pub fn one_child(&self, n: HuffmanNode) -> HuffmanNode {
    self.nodes[n+1]
  }
}

pub fn from_bit_lengths(bit_lengths: &[u8]) -> Result<~HuffmanTree,~DeflateError> {
  let mut bl_count: ~[int] = ~[0];
  let mut bl_syms: ~[~[u16]] = ~[~[]];
  let mut max_bl: uint = 0;

  for vec::eachi(bit_lengths) |sym, bl_ref| {
    let bl = *bl_ref as uint;
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

  let mut nodes: ~[u16] = ~[0xdead, 0xdead];
  let mut front: ~[u16] = ~[0];
  let mut front_offset: uint = 0;

  for uint::range(1, max_bl+1) |level| {
    let mut new_front: ~[u16] = ~[];

    for uint::range(front_offset, front.len()) |i| {
      let front_n: u16 = front[i];
      let zero_n: u16 = nodes.len() as u16;
      let one_n: u16 = zero_n + 2;
      nodes[front_n] = zero_n;
      nodes[front_n+1] = one_n;

      vec::grow(&mut nodes, 4, &0xdead);
      new_front.push(zero_n);
      new_front.push(one_n);
    }

    front = new_front;
    front_offset = 0;

    if bl_syms[level].len() <= front.len() {
      for bl_syms[level].each |&sym| {
        let sym_n = front[front_offset];
        nodes[sym_n] = 0;
        nodes[sym_n+1] = sym;

        front_offset += 1;
      }
    } else {
      return Err(~TooManyHuffCodesError(level));
    }
  }

  if front_offset >= front.len() {
    Ok(~HuffmanTree { nodes: nodes })
  } else {
    Err(~MissingHuffCodesError(max_bl))
  }
}

#[cfg(test)]
mod test {
  use deflate::huffman_tree::*;
  use deflate::error::*;

#[test]
  fn test_huffman_tree() {
    let a = 10, b = 20, c = 100, d = 42, e = 70, f = 333;
    let tree = HuffmanTree { nodes: ~[
      2, 8, 4, 6, 0, a, 0, b, 10, 16, 12, 14, 0, c, 0, d, 18, 20, 0, e, 0, f
    ]};

    let n = tree.root();
    assert!(!tree.is_leaf(n));

    let n0 = tree.zero_child(n);
    assert!(!tree.is_leaf(n0));

    let n00 = tree.zero_child(n0);
    assert!(tree.is_leaf(n00));
    assert_eq!(tree.leaf_value(n00), a);

    let n01 = tree.one_child(n0);
    assert!(tree.is_leaf(n01));
    assert_eq!(tree.leaf_value(n01), b);

    let n10 = tree.zero_child(tree.one_child(n));
    assert!(!tree.is_leaf(n10));

    let n101 = tree.one_child(n10);
    assert!(tree.is_leaf(n101));
    assert_eq!(tree.leaf_value(n101), d);
  }

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

    let tree = from_bit_lengths(bit_lengths).unwrap();

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
    match from_bit_lengths(~[2,2,0,3,3,3,2,0]) {
      Err(~TooManyHuffCodesError(3)) => { /* ok */ },
      Err(err) => fail!(fmt!("expected TooManyHuffCodesError, got %s", err.to_str())),
      Ok(_) => fail!(~"expected an error")
    }

    /* not all 4 codes are used */
    match from_bit_lengths(~[3,3,3,4,3,3,4,3]) {
      Err(~MissingHuffCodesError(4)) => { /* ok */ },
      Err(err) => fail!(fmt!("expected MissingHuffCodesError, got %s", err.to_str())),
      Ok(_) => fail!(~"expected an error")
    }
  }
}
