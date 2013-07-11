use gzip::hdr_decoder;
use gzip::body_decoder;
use gzip::header;
use gzip::error;

pub fn decode_hdr_ok(bytes: &[u8]) -> ~header::Header {
  let decoder = hdr_decoder::HeaderDecoder::new();
  match decoder.input(bytes) {
    Right((Ok(hdr), [])) => hdr,
    other => fail!(fmt!("decode_hdr_ok: unexpected %?", other)),
  }
}

pub fn decode_hdr_err<'a>(bytes: &'a [u8]) -> (~error::Error, &'a [u8]) {
  let decoder = hdr_decoder::HeaderDecoder::new();
  match decoder.input(bytes) {
    Right((Err(err), rest)) => (err, rest),
    other => fail!(fmt!("decode_hdr_err: unexpected %?", other)),
  }
}

pub fn decode_body_ok(bytes: &[u8]) -> ~[u8] {
  let decoder = body_decoder::BodyDecoder::new();
  match decoder.input(bytes, ~[]) {
    (Right((Ok(()), [])), data) => data,
    other => fail!(fmt!("decode_body_ok: unexpected %?", other)),
  }
}

pub fn decode_body_err<'a>(bytes: &'a [u8]) -> (~error::Error, &'a [u8]) {
  let decoder = body_decoder::BodyDecoder::new();
  match decoder.input(bytes, ()) {
    (Right((Err(err), rest)), ()) => (err, rest),
    other => fail!(fmt!("decode_body_err: unexpected %?", other)),
  }
}
