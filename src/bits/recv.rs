use std::vec;

pub trait Recv<X> {
  pub fn receive(self, xs: &[X]) -> Self;
}

pub struct SplitRecv<X, L, R> {
  priv left: L,
  priv right: R,
}

impl<X, L: Recv<X>, R: Recv<X>> Recv<X> for (L, R) {
  pub fn receive(self, xs: &[X]) -> (L, R) {
    let (left, right) = self;
    (left.receive(xs), right.receive(xs))
  }
}

impl<X: Copy> Recv<X> for ~[X] {
  pub fn receive(self, xs: &[X]) -> ~[X] {
    vec::append(self, xs)
  }
}

impl<'self, X> Recv<X> for &'self fn(&[X]) {
  pub fn receive(self, xs: &[X]) -> &'self fn(&[X]) {
    (self)(xs); self
  }
}

impl<X> Recv<X> for () {
  pub fn receive(self, elems: &[X]) -> () {
    let _ = elems;
  }
}

#[cfg(test)]
mod test {

  #[test]
  fn test_pair_recv() {
    let recv = (~[], ~[]);
    let recv = recv.receive(&[1, 1, 2, 3]);
    let recv = recv.receive(&[5, 8, 13]);
    let (bufA, bufB) = recv;

    assert_eq!(bufA, ~[1, 1, 2, 3, 5, 8, 13]);
    assert_eq!(bufB, ~[1, 1, 2, 3, 5, 8, 13]);
  }

  #[test]
  fn test_vec_recv() {
    let buf = ~[];
    let buf = buf.receive(&['a', 'b', 'c']);
    let buf = buf.receive(&['d', 'e']);

    assert_eq!(buf, ~['a', 'b', 'c', 'd', 'e']);
  }

  #[test]
  fn test_fn_recv() {
    let mut buf = ~[];
    let fun: &fn(&[bool]) = |xs| buf.push_all(xs);
    let fun = fun.receive(&[true, false, true]);
    let fun = fun.receive(&[true, true]);
    let _ = fun;

    assert_eq!(buf, ~[true, false, true, true, true]);
  }

  #[test]
  fn test_unit_recv() {
    let unit_recv = ();
    let _ = unit_recv.receive(&[1, 2, 3]);
  }
}
