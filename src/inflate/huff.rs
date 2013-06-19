use inflate::error;

pub struct Tree {
  priv x: uint,
}

struct Node(u16);

impl Tree {
  pub fn new_empty() -> Tree {
    fail!()
  }

  pub fn new_from_lens(lens: &[u8]) -> Result<Tree, ~error::Error> {
    fail!()
  }

  pub fn root(&self) -> Node {
    fail!();
  }

  pub fn zero_child(&self, n: Node) -> Node {
    fail!();
  }

  pub fn one_child(&self, n: Node) -> Node {
    fail!();
  }

  pub fn is_leaf(&self, n: Node) -> bool {
    fail!();
  }

  pub fn leaf_value(&self, n: Node) -> u16 {
    fail!();
  }

  pub fn is_defined(&self, n: Node) -> bool {
    fail!();
  }
}

pub static undefined_leaf_value: u16 = 0;

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
