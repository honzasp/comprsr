use bits;
use inflate::error;
use std::{vec, uint};

pub struct Output<R> {
  priv recv: R,
  priv window: ~[u8],
  priv wrapped: bool,
  priv pos: uint,
  priv cache_pos: uint,
}

impl<R: bits::recv::Recv<u8>> Output<R> {
  pub fn new(window_size: uint, recv: R) -> Output<R> {
    Output {
      recv: recv,
      window: vec::from_elem(window_size, 77),
      wrapped: false,
      pos: 0, cache_pos: 0,
    }
  }

  pub fn close(self) -> R {
    self.recv
  }

  pub fn send_literal_chunk(&mut self, chunk: &[u8]) {
    self.flush_cache();

    let mut chunk_rest = chunk;
    loop {
      let window_free = self.window.len() - self.pos;

      if chunk_rest.len() <= window_free {
        for uint::iterate(0, chunk_rest.len()) |i| {
          self.window[self.pos + i] = chunk_rest[i];
        }
        self.pos = self.pos + chunk_rest.len();
        break;
      } else {
        for uint::iterate(0, window_free) |i| {
          self.window[self.pos + i] = chunk_rest[i];
        }
        self.pos = 0;
        self.wrapped = true;
        chunk_rest = chunk_rest.slice(window_free, chunk_rest.len());
      }
    }

    self.cache_pos = self.pos;
    self.recv = self.recv.receive(chunk);
  }

  pub fn send_literal(&mut self, byte: u8) {
    if self.pos >= self.window.len() {
      self.flush_cache();
      self.pos = 0;
      self.cache_pos = 0;
      self.wrapped = true;
    }
    self.window[self.pos] = byte;
    self.pos = self.pos + 1;
  }

  pub fn back_reference(&mut self, dist: uint, len: uint)
    -> Result<(),~error::Error>
  {
    if !self.wrapped && dist > self.pos {
      Err(~error::ReferenceBeforeStart(dist, len, self.pos))
    } else if dist > self.window.len() {
      Err(~error::ReferenceOutOfWindow(dist, len, self.window.len()))
    } else {
      let mut back_pos = if dist > self.pos {
          self.window.len() + self.pos - dist
        } else {
          self.pos - dist
        };

      for len.times {
        if self.pos >= self.window.len() {
          self.flush_cache();
          self.pos = 0;
          self.cache_pos = 0;
          self.wrapped = true;
        }

        if back_pos >= self.window.len() {
          back_pos = 0;
        }

        self.window[self.pos] = self.window[back_pos];
        self.pos = self.pos + 1;
        back_pos = back_pos + 1;
      }

      Ok(())
    }
  }

  pub fn flush(&mut self) {
    self.flush_cache();
  }

  priv fn flush_cache(&mut self) {
    self.recv = self.recv.receive(self.window.slice(self.cache_pos, self.pos));
    self.cache_pos = self.pos;
  }
}

#[cfg(test)]
mod test {
  use inflate::test_helpers::*;
  use inflate::out::*;

  #[test]
  fn test_send_literal() {
    let buf: ~[u8] = ~[];
    let mut out = Output::new(10, buf);
    out.send_literal(10);
    out.send_literal(20);
    out.send_literal(30);
    out.flush();
    let buf = out.close();

    assert_eq!(buf, ~[10, 20, 30]);
  }

  #[test]
  fn test_send_literal_chunk() {
    {
      let buf: ~[u8] = ~[];
      let mut out = Output::new(10, buf);

      out.send_literal_chunk(&[1,2,3]);
      out.send_literal_chunk(&[4,5]);
      out.send_literal_chunk(&[6,7,8,9]);
      out.flush();

      let buf = out.close();
      assert_eq!(buf, ~[1,2,3,4,5,6,7,8,9]);
    }

    { // wrap the window
      let buf: ~[u8] = ~[];
      let mut out = Output::new(5, buf);

      out.send_literal_chunk(&[1,2,3]);
      out.send_literal_chunk(&[4,5,6,7,8]);
      out.send_literal_chunk(&[9,10,11,12,13,14,15,16,17,18,19,20]);
      out.flush();

      let buf = out.close();
      assert_eq!(buf, ~[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20]);
    }
  }

  #[test]
  fn test_back_reference() {
    {
      let buf: ~[u8] = ~[];
      let mut out = Output::new(8, buf);

      out.send_literal_chunk(&[2,3,5,7]);
      out.send_literal(11);
      assert_eq!(out.back_reference(2, 5), Ok(()));
      out.flush();

      let buf = out.close();
      assert_eq!(buf, ~[2,3,5,7,11,7,11,7,11,7]);
    };

    { // window wrapped
      let buf: ~[u8] = ~[];
      let mut out = Output::new(8, buf);

      out.send_literal_chunk(&[2,3,5]);
      out.send_literal_chunk(&[7,11,13,17]);
      out.send_literal_chunk(&[19,23,29,31]);
      assert_eq!(out.back_reference(5, 4), Ok(()));
      out.flush();

      let buf = out.close();
      assert_eq!(buf, ~[2,3,5,7,11,13,17,19,23,29,31,17,19,23,29]);
    };

    { // maximal distance
      let buf: ~[u8] = ~[];
      let mut out = Output::new(4, buf);

      out.send_literal_chunk(&[2,3,5,7]);
      assert_eq!(out.back_reference(4,6), Ok(()));
      out.flush();

      let buf = out.close();
      assert_eq!(buf, ~[2,3,5,7,2,3,5,7,2,3]);
    };
  }

  #[test]
  fn test_back_reference_errors() {
    { // dist too long (window not full)
      let buf: ~[u8] = ~[];
      let mut out = Output::new(5, buf);

      out.send_literal_chunk(&[1,2,3]);
      assert_eq!(out.back_reference(4, 2),
        Err(~error::ReferenceBeforeStart(4, 2, 3)));
      out.send_literal_chunk(&[4,5,6]);
      out.flush();

      let buf = out.close();
      assert_eq!(buf, ~[1,2,3,4,5,6]);
    }

    { // dist too long (longer than the window)
      let buf: ~[u8] = ~[];
      let mut out = Output::new(5, buf);

      out.send_literal_chunk(&[1,2,3,4,5,6,7,8]);
      assert_eq!(out.back_reference(8, 2),
        Err(~error::ReferenceOutOfWindow(8, 2, 5)));
      out.send_literal_chunk(&[9,10,11]);
      out.flush();

      let buf = out.close();
      assert_eq!(buf, ~[1,2,3,4,5,6,7,8,9,10,11]);
    }
  }
}
