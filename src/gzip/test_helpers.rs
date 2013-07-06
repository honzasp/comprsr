use gzip::decoder;
use gzip::header;
use gzip::error;

pub fn decode_ok(bytes: &[u8]) -> ~[(~header::Header, ~[u8])] {
  let recv: ~[(~header::Header, ~[u8])] = ~[];
  let mut decoder = decoder::Decoder::new(~recv);

  match decoder.input(bytes) {
    decoder::ConsumedRes => { },
    decoder::ErrorRes(err, _rest) =>
      fail!(fmt!("decode_ok: unexpected error from input: %?", err)),
  };

  match decoder.finish() {
    decoder::FinishedRes => { },
    decoder::ErrorFinRes(err) =>
      fail!(fmt!("decode_ok: unexpected error from finish: %?", err)),
  };

  *decoder.close()
}

pub fn decode_ok1(bytes: &[u8]) -> (~header::Header, ~[u8]) {
  match decode_ok(bytes) {
    [member] => member,
    other => fail!(fmt!("decode_ok1: got %?", other)),
  }
}

pub fn decode_err<'a>(bytes: &'a [u8]) -> (~error::Error, Option<&'a [u8]>) {
  let recv = ();
  let mut decoder = decoder::Decoder::new(~recv);

  match decoder.input(bytes) {
    decoder::ErrorRes(err, rest) => (err, Some(rest)),
    decoder::ConsumedRes => 
      match decoder.finish() {
        decoder::ErrorFinRes(err) => (err, None),
        decoder::FinishedRes =>
          fail!(fmt!("decode_err: did not get any error")),
      },
  }
}

