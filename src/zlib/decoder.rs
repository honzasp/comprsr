use recv;
use inflate::inflater;
use checksums::adler32;
use zlib::error;
use std::cmp;
use std::uint;

struct Decoder<R> {
  priv stage: Stage<R>,
  priv waiting: ~[u8],
  priv opt_recv: Option<~R>,
  priv opt_infl: Option<~inflater::Inflater<
      recv::ForkReceiver<u8, R, adler32::Adler32>
    >>,
}

#[deriving(Eq)]
pub enum Res<A> {
  pub ConsumedRes(),
  pub FinishedRes(A),
  pub ErrorRes(~error::Error, A),
}

#[deriving(ToStr)]
enum Stage<R> {
  HeaderStage,
  DataStage,
  Adler32Stage(u32),
  ErrorStage(~error::Error),
  EndStage,
}

impl<R: recv::Receiver<u8>> Decoder<R> {
  pub fn new(receiver: ~R) -> Decoder<R> {
    Decoder { 
      stage: HeaderStage,
      waiting: ~[],
      opt_recv: Some(receiver),
      opt_infl: None,
    }
  }

  pub fn close(self) -> ~R {
    if self.opt_recv.is_some() {
      if self.opt_infl.is_some() {
        fail!(~"Both self.opt_recv and self.opt_infl are Some");
      } else {
        self.opt_recv.unwrap()
      }
    } else {
      if self.opt_infl.is_some() {
        let (recv, _a32) = self.opt_infl.unwrap().close().close();
        recv
      } else {
        fail!(~"Neither self.opt_recv nor self.opt_infl are Some");
      }
    }
  }

  pub fn input<'a>(&mut self, chunk: &'a [u8]) -> Res<&'a [u8]> {
    let mut rest = chunk;
    loop {
      self.stage = match self.stage {
        HeaderStage => {
          let to_wait = cmp::min(rest.len(), 2 - self.waiting.len());
          for uint::range(0, to_wait) |i| {
            self.waiting.push(rest[i]);
          }
          rest = rest.slice(to_wait, rest.len());

          if self.waiting.len() >= 2 {
            let cmf = self.waiting[0];
            let flg = self.waiting[1];

            self.waiting = ~[];

            let cm = cmf & 0b1111;
            let cinfo = (cmf >> 4) & 0b1111;

            let _fcheck = flg & 0b11111;
            let fdict = (flg >> 5) & 0b1;
            let _flevel = (flg >> 6) & 0b11;

            let win_size: uint = 1 << (8 + cinfo as uint);

            if cm != 8 {
              ErrorStage(~error::BadCompressionMethod(cm as uint))
            } else if win_size > 32 * 1024 {
              ErrorStage(~error::WindowTooLong(win_size))
            } else if (cmf as uint * 256 + flg as uint) % 31 != 0 {
              ErrorStage(~error::BadHeaderChecksum(cmf, flg))
            } else if fdict != 0 {
              ErrorStage(~error::DictionaryUsed)
            } else {
              // unique type dance :)
              if self.opt_recv.is_some() {
                let recv = self.opt_recv.swap_unwrap();
                let a32 = ~adler32::Adler32::new();
                let fork_recv = ~recv::ForkReceiver::new(recv, a32);
                let inflater = ~inflater::Inflater::new(fork_recv);
                self.opt_infl = Some(inflater);
                DataStage
              } else {
                fail!(fmt!("Decoder.input: stage is HeaderStage, \
                  but self.opt_recv is None"));
              }
            }
          } else {
            return ConsumedRes
          }
        },
        DataStage => {
          match self.opt_infl.get_mut_ref().input(rest) {
            inflater::ConsumedRes => 
              return ConsumedRes,
            inflater::ErrorRes(error, inflate_rest) => {
              rest = inflate_rest;
              let inflater = self.opt_infl.swap_unwrap();
              let (recv, _a32) = inflater.close().close();
              self.opt_recv = Some(recv);
              ErrorStage(~error::InflateError(error))
            },
            inflater::FinishedRes(inflate_rest) => {
              rest = inflate_rest;
              let inflater = self.opt_infl.swap_unwrap();
              let (recv, a32) = inflater.close().close();
              self.opt_recv = Some(recv);
              Adler32Stage(a32.adler32())
            },
          }
        },
        Adler32Stage(expected_a32) => {
          let to_wait = cmp::min(rest.len(), 4 - self.waiting.len());
          for uint::range(0, to_wait) |i| {
            self.waiting.push(rest[i]);
          }
          rest = rest.slice(to_wait, rest.len());

          if self.waiting.len() >= 4 {
            let read_a32 =
              (self.waiting[0] as u32 << 24) |
              (self.waiting[1] as u32 << 16) |
              (self.waiting[2] as u32 << 8) |
              (self.waiting[3] as u32);
            self.waiting = ~[];

            if expected_a32 == read_a32 {
              EndStage
            } else {
              ErrorStage(~error::BadDataChecksum(expected_a32, read_a32))
            }
          } else {
            return ConsumedRes
          }
        },
        EndStage =>
          return FinishedRes(rest),
        ErrorStage(ref err) =>
          return ErrorRes(err.clone(), rest),
      }
    }
  }

  pub fn has_finished(&self) -> bool {
    match self.stage {
      EndStage      => true,
      ErrorStage(_) => true,
      _ => false,
    }
  }

  pub fn get_error(&self) -> Option<~error::Error> {
    match self.stage {
      ErrorStage(ref err) => Some(err.clone()),
      _ => None,
    }
  }

  pub fn is_error(&self) -> bool {
    self.get_error().is_some()
  }

  pub fn is_ready(&self) -> bool {
    !self.has_finished()
  }
}

#[cfg(test)]
mod test {
  use zlib::decoder;
  use zlib::error;
  use inflate;

  fn decode_ok(bytes: &[u8]) -> ~[u8] {
    let buf = ~[];
    let mut decoder = decoder::Decoder::new(~buf);

    match decoder.input(bytes) {
      decoder::FinishedRes(rest) if rest.is_empty() => *decoder.close(),
      x => fail!(fmt!("decode_ok: unexpected Res %?", x)),
    }
  }

  fn decode_err<'a>(bytes: &'a [u8]) -> (~error::Error, &'a [u8]) {
    let receiver = ();
    let mut decoder = decoder::Decoder::new(~receiver);

    match decoder.input(bytes) {
      decoder::ErrorRes(err, rest) => (err, rest),
      x => fail!(fmt!("decode_err: unexpected Res %?", x)),
    }
  }

  #[test]
  fn test_decode_ok() {
    assert_eq!(decode_ok(&[
        0b01111000, 0b10011100, 0b01100011, 0b01100100, 0b01100010,
        0b00000110, 0b00000000, 0b00000000, 0b00001101, 0b00000000,
        0b00000111
      ]),
      ~[1, 2, 3]
    );

    assert_eq!(decode_ok(&[
        0b01111000, 0b10011100, 0b01100011, 0b01100010, 0b01100110,
        0b01100101, 0b11100111, 0b00000110, 0b00000000, 0b00000000,
        0b01000011, 0b00000000, 0b00011101
      ]),
      ~[2, 3, 5, 7, 11]
    );

    assert_eq!(decode_ok(&[
        0b01111000, 0b10011100, 0b11101011, 0b10101001, 0b01101001,
        0b10011000, 0b00110001, 0b10100001, 0b10100111, 0b10100111,
        0b01100110, 0b01001010, 0b01000011, 0b01001101, 0b01001011,
        0b11000111, 0b10000100, 0b10011110, 0b00111001, 0b00101101,
        0b00001101, 0b00111101, 0b01110011, 0b00000000, 0b01110000,
        0b00101100, 0b00001010, 0b11000101
      ]),
      ~[140, 124, 128, 152, 144, 140, 140, 124,
        148, 128, 124, 132, 136, 144, 140, 156,
        132, 128, 140, 156]
    );
  }

  #[test]
  fn test_decode_err() {
    assert_eq!(decode_err(&[
        0b01111001, 0b10011100, 0b01100011, 0b01100100, 0b01100010,
      ]),
      (~error::BadCompressionMethod(0b1001),
        &[0b01100011, 0b01100100, 0b01100010])
    );

    assert_eq!(decode_err(&[
        0b10001000, 0b10011100, 0b01100011, 0b01100100, 0b01100010,
      ]),
      (~error::WindowTooLong(64 * 1024), 
        &[0b01100011, 0b01100100, 0b01100010])
    );

    assert_eq!(decode_err(&[
        0b01111000, 0b10111011, 0b01100011, 0b01100100, 0b01100010
      ]),
      (~error::DictionaryUsed,
        &[0b01100011, 0b01100100, 0b01100010])
    );

    assert_eq!(decode_err(&[
        0b01111000, 0b10011101, 0b01100011, 0b01100100, 0b01100010,
      ]),
      (~error::BadHeaderChecksum(0b01111000, 0b10011101),
        &[0b01100011, 0b01100100, 0b01100010])
    );

    assert_eq!(decode_err(&[
        0b01111000, 0b10011100, 0b01100111, 0b01100100, 0b01100010,
      ]),
      (~error::InflateError(~inflate::error::BadBlockType(0b11)),
        &[0b01100100, 0b01100010])
    );

    assert_eq!(decode_err(&[
        0b01111000, 0b10011100, 0b01100011, 0b01100010, 0b01100110,
        0b01100101, 0b11100111, 0b00000110, 0b00000000, 0b00000000,
        0b01000011, 0b11100000, 0b00011101, 7, 8, 9,
      ]),
      (~error::BadDataChecksum(
          0b00000000_01000011_00000000_00011101,
          0b00000000_01000011_11100000_00011101
        ), &[7, 8, 9])
    );
  }

}
