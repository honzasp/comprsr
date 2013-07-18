use bits;
use inflate::error;
use std::{vec, uint};

pub struct Output {
  priv window: ~[u8],
  priv wrapped: bool,
  priv pos: uint,
  priv cache_pos: uint,
}

impl Output {
  pub fn new(window_size: uint) -> Output {
    Output {
      window: vec::from_elem(window_size, 77u8),
      wrapped: false,
      pos: 0, cache_pos: 0,
    }
  }

  pub fn send_literal_chunk<R: bits::recv::Recv<u8>>
    (&mut self, chunk: &[u8], recv: R) -> R 
  {
    let recv = self.flush_cache(recv);

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
    recv.receive(chunk)
  }

  pub fn send_literal<R: bits::recv::Recv<u8>>
    (&mut self, byte: u8, recv: R) -> R 
  {
    let mut recv = recv;

    if self.pos >= self.window.len() {
      recv = self.flush_cache(recv);
      self.pos = 0;
      self.cache_pos = 0;
      self.wrapped = true;
    }

    self.window[self.pos] = byte;
    self.pos = self.pos + 1;
    recv
  }

  pub fn back_reference<R: bits::recv::Recv<u8>>
    (&mut self, dist: uint, len: uint, recv: R)
    -> (Result<(),~error::Error>, R)
  {
    if !self.wrapped && dist > self.pos {
      (Err(~error::ReferenceBeforeStart(dist, len, self.pos)), recv)
    } else if dist > self.window.len() {
      (Err(~error::ReferenceOutOfWindow(dist, len, self.window.len())), recv)
    } else {
      let mut recv = recv;

      let mut back_pos = if dist > self.pos {
          self.window.len() + self.pos - dist
        } else {
          self.pos - dist
        };

      // TODO: the `for` causes "error: cannot move out of captured outer variable"
      // for len.times {
      let mut i = 0;
      while i < len {
        i = i + 1;

        if self.pos >= self.window.len() {
          recv = self.flush_cache(recv);
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

      (Ok(()), recv)
    }
  }

  pub fn flush<R: bits::recv::Recv<u8>>
    (&mut self, recv: R) -> R 
  {
    self.flush_cache(recv)
  }

  priv fn flush_cache<R: bits::recv::Recv<u8>>
    (&mut self, recv: R) -> R 
  {
    let recv = recv.receive(self.window.slice(self.cache_pos, self.pos));
    self.cache_pos = self.pos;
    recv
  }
}

#[cfg(test)]
mod test {
  use inflate::test_helpers::*;
  use inflate::out::*;

  #[test]
  fn test_send_literal() {
    let mut out = Output::new(10);

    let buf: ~[u8] = ~[];
    let buf = out.send_literal(10, buf);
    let buf = out.send_literal(20, buf);
    let buf = out.send_literal(30, buf);
    let buf = out.flush(buf);

    assert_eq!(buf, ~[10, 20, 30]);
  }

  #[test]
  fn test_send_literal_chunk() {
    {
      let mut out = Output::new(10);

      let buf: ~[u8] = ~[];
      let buf = out.send_literal_chunk(&[1,2,3], buf);
      let buf = out.send_literal_chunk(&[4,5], buf);
      let buf = out.send_literal_chunk(&[6,7,8,9], buf);
      let buf = out.flush(buf);

      assert_eq!(buf, ~[1,2,3,4,5,6,7,8,9]);
    }

    { // wrap the window
      let mut out = Output::new(5);

      let buf: ~[u8] = ~[];
      let buf = out.send_literal_chunk(&[1,2,3], buf);
      let buf = out.send_literal_chunk(&[4,5,6,7,8], buf);
      let buf = out.send_literal_chunk(
        &[9,10,11,12,13,14,15,16,17,18,19,20], buf);
      let buf = out.flush(buf);

      assert_eq!(buf, ~[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20]);
    }
  }

  #[test]
  fn test_back_reference() {
    {
      let mut out = Output::new(8);

      let buf: ~[u8] = ~[];
      let buf = out.send_literal_chunk(&[2,3,5,7], buf);
      let buf = out.send_literal(11, buf);
      let (res, buf) = out.back_reference(2, 5, buf);
      assert_eq!(res, Ok(()));
      let buf = out.flush(buf);

      assert_eq!(buf, ~[2,3,5,7,11,7,11,7,11,7]);
    };

    { // window wrapped
      let mut out = Output::new(8);

      let buf: ~[u8] = ~[];
      let buf = out.send_literal_chunk(&[2,3,5], buf);
      let buf = out.send_literal_chunk(&[7,11,13,17], buf);
      let buf = out.send_literal_chunk(&[19,23,29,31], buf);
      let (res, buf) = out.back_reference(5, 4, buf);
      assert_eq!(res, Ok(()));
      let buf = out.flush(buf);

      assert_eq!(buf, ~[2,3,5,7,11,13,17,19,23,29,31,17,19,23,29]);
    };

    { // maximal distance
      let mut out = Output::new(4);

      let buf: ~[u8] = ~[];
      let buf = out.send_literal_chunk(&[2,3,5,7], buf);
      let (res, buf) = out.back_reference(4, 6, buf);
      assert_eq!(res, Ok(()));
      let buf = out.flush(buf);

      assert_eq!(buf, ~[2,3,5,7,2,3,5,7,2,3]);
    };
  }

  #[test]
  fn test_back_reference_errors() {
    { // dist too long (window not full)
      let mut out = Output::new(5);

      let buf: ~[u8] = ~[];
      let buf = out.send_literal_chunk(&[1,2,3], buf);
      let (res, buf) = out.back_reference(4, 2, buf);
      assert_eq!(res, Err(~error::ReferenceBeforeStart(4, 2, 3)));
      let buf = out.send_literal_chunk(&[4,5,6], buf);
      let buf = out.flush(buf);

      assert_eq!(buf, ~[1,2,3,4,5,6]);
    }

    { // dist too long (longer than the window)
      let mut out = Output::new(5);

      let buf: ~[u8] = ~[];
      let buf = out.send_literal_chunk(&[1,2,3,4,5,6,7,8], buf);
      let (res, buf) = out.back_reference(8, 2, buf);
      assert_eq!(res, Err(~error::ReferenceOutOfWindow(8, 2, 5)));
      let buf = out.send_literal_chunk(&[9,10,11], buf);
      let buf = out.flush(buf);

      assert_eq!(buf, ~[1,2,3,4,5,6,7,8,9,10,11]);
    }
  }
}
