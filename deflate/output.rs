use deflate::bit_reader::{BitReader};
use deflate::error::*;

pub struct Output {
  writer: @io::Writer,
  len: uint,
  win: ~[u8],
  win_pos: uint
}

impl Output {
  pub fn new(writer: @io::Writer) -> ~Output {
    let mut win = ~[];
    vec::reserve(&mut win, 32_768);
    vec::grow(&mut win, 32_768, &0);

    ~Output { writer: writer, win: win, win_pos: 0, len: 0 }
  }

  pub fn window_len(&self) -> uint {
    cmp::min(self.len, self.win.len())
  }

  pub fn len(&self) -> uint {
    self.len
  }

  pub fn copy_bytes(&mut self, len: uint, in: &mut BitReader) 
    -> Option<~DeflateError> 
  {
    if len >= self.win.len() {
      let e1 = do in.read_bytes(len - self.win.len()) |bytes| {
        self.writer.write(bytes);
      };

      if e1.is_some() { return e1; }

      self.win_pos = 0;
      self.len = self.len + len;

      do in.read_bytes(self.win.len()) |bytes| {
        for bytes.each |&b| {
          self.win[self.win_pos] = b;
          self.win_pos = self.win_pos + 1;
        }
      }
    } else {
      do in.read_bytes(len) |bytes| {
        for bytes.each |&b| {
          self.write(b);
        }
      }
    }
  }

  pub fn repeat(&mut self, len: uint, dist: uint) {
    let mut back_pos = if self.win_pos > dist {
        self.win_pos - dist
      } else {
        self.win.len() + self.win_pos - dist
      };
    let mut rem = len;

    while rem > 0 {
      if self.win_pos >= self.win.len() {
        self.win_pos = 0;
      }

      if back_pos >= self.win.len() {
        back_pos = 0;
      }

      let byte = self.win[back_pos];
      self.win[self.win_pos] = byte;
      self.writer.write(&[byte]);

      rem = rem - 1;
      self.win_pos = self.win_pos + 1;
      back_pos = back_pos + 1;
    }

    self.len = self.len + len;
  }

  pub fn write(&mut self, byte: u8) {
    if self.win_pos >= self.win.len() {
      self.win_pos = 0;
    }
    self.win[self.win_pos] = byte;
    self.win_pos = self.win_pos + 1;
    self.len = self.len + 1;
    self.writer.write(&[byte]);
  }
}
