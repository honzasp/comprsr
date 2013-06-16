use std::vec;

pub struct Output<'self> {
  priv callback: &'self fn(&[u8]),
  priv window: ~[u8],
}

impl<'self> Output<'self> {
  pub fn new<'a>(callback: &'a fn(&[u8]), window_size: uint) -> Output<'a> {
    Output { callback: callback, window: vec::from_elem(window_size, 77) }
  }

  pub fn send_literal_chunk(&mut self, chunk: &[u8]) {
    (self.callback)(chunk);
  }
}
