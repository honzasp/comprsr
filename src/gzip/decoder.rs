use gzip::error;
use gzip::member_recv;

pub struct Decoder<R> {
  priv x: (),
}

// TODO: move the common part to crate "bits"?
#[deriving(Eq)]
pub enum Res<A> {
  pub ConsumedRes(),
  pub FinishedRes(A),
  pub ErrorRes(~error::Error, A),
}

impl<R: member_recv::MemberReceiver<S>, S> Decoder<R> {
  pub fn new(_member_recv: ~R) -> Decoder<R> {
    fail!();
  }

  pub fn close(self) -> ~R {
    fail!();
  }

  pub fn input<'a>(&mut self, _chunk: &'a [u8]) -> Res<&'a [u8]> {
    fail!();
  }

  pub fn has_finished(&self) -> bool {
    fail!();
  }

  pub fn get_error(&self) -> Option<~error::Error> {
    fail!();
  }

  pub fn is_error(&self) -> bool {
    fail!();
  }

  pub fn is_ready(&self) -> bool {
    !self.has_finished()
  }
}

#[cfg(test)]
mod test {
  use gzip::decoder;
  use gzip::header;
  use gzip::error;

  fn decode_ok(bytes: &[u8]) -> ~[(~header::Header, ~[u8])] {
    let recv: ~[(~header::Header, ~[u8])] = ~[];
    let mut decoder = decoder::Decoder::new(~recv);

    match decoder.input(bytes) {
      decoder::FinishedRes(rest) if rest.is_empty() => *decoder.close(),
      x => fail!(fmt!("decode_ok: unexpected Res %?", x)),
    }
  }

  fn decode_err<'a>(bytes: &'a [u8]) -> (~error::Error, &'a [u8]) {
    let recv = ();
    let mut decoder = decoder::Decoder::new(~recv);

    match decoder.input(bytes) {
      decoder::ErrorRes(err, rest) => (err, rest),
      x => fail!(fmt!("decode_err: unexpected Res %?", x)),
    }
  }

  fn header(f: &fn(&mut header::Header)) -> ~header::Header {
    let mut header = ~header::Header::empty();
    f(header);
    header
  }

  #[test]
  fn test_decode_ok() {
    {
      let hdr = do header |h| {
        h.extra_flags = 0;
        h.mtime = Some(0x87654321);
        h.system = Some(header::Amiga);
      };

      assert_eq!(decode_ok(&[
          0x1f, 0x8b, 0x08, 0x00, 0x21, 0x43, 0x65, 0x87,
          0x00, 0x01, 0xe3, 0x12, 0x91, 0x03, 0x00, 0xf2,
          0xb6, 0x77, 0x26, 0x03, 0x00, 0x00, 0x00
        ]),
        ~[(hdr, ~[10, 20, 30])]
      );
    }
  }
}
