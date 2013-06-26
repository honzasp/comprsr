use inflate::inflater;
//use checksums::adler32;
use zlib::error;
use std::cmp;
use std::uint;

struct Decoder<'self> {
  priv stage: Stage<'self>,
  priv callback: &'self fn(&[u8]),
  priv waiting: ~[u8],
}

#[deriving(Eq)]
pub enum Res<A> {
  pub ConsumedRes(),
  pub FinishedRes(A),
  pub ErrorRes(~error::Error, A),
}

enum Stage<'self> {
  HeaderStage,
  DataStage(inflater::Inflater<'self>),
  Adler32Stage,
  EndStage,
}

impl<'self> Decoder<'self> {
  pub fn new<'a>(callback: &'a fn(&[u8])) -> Decoder<'a> {
    Decoder { 
      stage: HeaderStage,
      callback: callback,
      waiting: ~[],
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
              return ErrorRes(~error::BadCompressionMethod(cm as uint), rest);
            } else if win_size > 32 * 1024 {
              return ErrorRes(~error::WindowTooLong(win_size), rest);
            } else if (cmf as uint * 256 + flg as uint) % 31 != 0 {
              return ErrorRes(~error::BadHeaderChecksum(cmf, flg), rest);
            } else if fdict != 0 {
              return ErrorRes(~error::DictionaryUsed, rest);
            } else {
              let inflater = inflater::Inflater::new(self.callback);
              DataStage(inflater)
            }
          } else {
            return ConsumedRes
          }
        },
        DataStage(ref mut inflater) => 
          match inflater.input(rest) {
            inflater::ConsumedRes => 
              return ConsumedRes,
            inflater::ErrorRes(error, inflate_rest) => 
              return ErrorRes(~error::InflateError(error), inflate_rest),
            inflater::FinishedRes(inflate_rest) => {
              rest = inflate_rest;
              Adler32Stage
            },
          },
        Adler32Stage => {
          let to_wait = cmp::min(rest.len(), 4 - self.waiting.len());
          for uint::range(0, to_wait) |i| {
            self.waiting.push(rest[i]);
          }
          rest = rest.slice(to_wait, rest.len());

          if self.waiting.len() >= 4 {
            // TODO: check the checksum
            EndStage
          } else {
            return ConsumedRes
          }
        },
        EndStage =>
          return FinishedRes(rest)
      }
    }
  }
}

#[cfg(test)]
mod test {
  use zlib::decoder;
  use zlib::error;
  use inflate;

  fn decode_ok(bytes: &[u8]) -> ~[u8] {
    let mut buf = ~[];
    let mut decoder = do decoder::Decoder::new |chunk| {
      buf.push_all(chunk);
    };

    match decoder.input(bytes) {
      decoder::FinishedRes(rest) if rest.is_empty() => buf,
      x => fail!(fmt!("decode_ok: unexpected Res %?", x)),
    }
  }

  fn decode_err<'a>(bytes: &'a [u8]) -> (~error::Error, &'a [u8]) {
    let mut decoder = do decoder::Decoder::new |_| { };

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
  }

}
