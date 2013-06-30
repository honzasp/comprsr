pub trait Receiver<A> {
  pub fn receive(&mut self, elems: &[A]);
}

struct FnReceiver<'self, A> {
  priv func: &'self fn(&[A]),
}

impl<'self, A> FnReceiver<'self, A> {
  #[inline]
  pub fn new<'a>(func: &'a fn(&[A])) -> FnReceiver<'a, A> {
    FnReceiver { func: func }
  }
}

impl<'self, A> Receiver<A> for FnReceiver<'self, A> {
  #[inline]
  pub fn receive(&mut self, elems: &[A]) {
    (self.func)(elems);
  }
}

struct ForkReceiver<A, RA, RB> {
  priv recvA: ~RA,
  priv recvB: ~RB,
}

impl<A, RA, RB> ForkReceiver<A, RA, RB> {
  #[inline]
  pub fn new(recvA: ~RA, recvB: ~RB) 
    -> ForkReceiver<A, RA, RB>
  {
    ForkReceiver { recvA: recvA, recvB: recvB }
  }

  #[inline]
  pub fn close(self) -> (~RA, ~RB) {
    match self {
      ForkReceiver { recvA: recvA, recvB: recvB } => (recvA, recvB)
    }
  }
}

impl<A, RA: Receiver<A>, RB: Receiver<A>>
  Receiver<A> for ForkReceiver<A, RA, RB>
{
  pub fn receive(&mut self, elems: &[A]) {
    self.recvA.receive(elems);
    self.recvB.receive(elems);
  }
}

impl<A: Copy> Receiver<A> for ~[A] {
  #[inline]
  pub fn receive(&mut self, elems: &[A]) {
    self.push_all(elems);
  }
}

impl<A> Receiver<A> for () {
  #[inline]
  pub fn receive(&mut self, elems: &[A]) {
    let _ = elems;
  }
}

#[cfg(test)]
mod test {
  use recv;

  #[test]
  fn test_fn_recv() {
    let mut buf = ~[];
    let callback: &fn(&[bool]) = |elems| buf.push(elems.to_owned());
    let mut fn_recv = recv::FnReceiver::new(callback);

    fn_recv.receive(&[true, false]);
    fn_recv.receive(&[]);
    fn_recv.receive(&[false, false, true]);
    fn_recv.receive(&[true]);

    assert_eq!(buf, ~[
        ~[true, false], ~[], ~[false, false, true], ~[true]
      ]);
  }

  #[test]
  fn test_fork_recv() {
    let bufA = ~[];
    let bufB = ~[];
    
    {
      let mut fork_recv = recv::ForkReceiver::new(~bufA, ~bufB);
      fork_recv.receive(&[1, 1, 2, 3]);
      fork_recv.receive(&[5, 8, 13]);
      let (bufA, bufB) = fork_recv.close();

      assert_eq!(bufA, ~~[1, 1, 2, 3, 5, 8, 13]);
      assert_eq!(bufB, ~~[1, 1, 2, 3, 5, 8, 13]);
    }
  }

  #[test]
  fn test_vec_recv() {
    let mut buf = ~[];

    buf.receive(&['a', 'b', 'c']);
    buf.receive(&['d', 'e']);

    assert_eq!(buf, ~['a', 'b', 'c', 'd', 'e']);
  }

  #[test]
  fn test_unit_recv() {
    let unit_recv = &mut ();
    unit_recv.receive(&[1, 2, 3]);
    unit_recv.receive(&[4, 5]);
  }
}
