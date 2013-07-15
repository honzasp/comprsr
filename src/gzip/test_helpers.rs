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

pub fn decode_hdr_chunked_ok(chunk_len: uint, bytes: &[u8]) -> ~header::Header {
  let mut decoder = hdr_decoder::HeaderDecoder::new();

  let mut iter = bytes.chunk_iter(chunk_len);
  loop {
    match iter.next() {
      Some(chunk) => {
        match decoder.input(chunk) {
          Left(new_decoder) => { decoder = new_decoder },
          Right((Ok(header), [])) => { return header },
          x => fail!(fmt!("decode_hdr_chunked_ok: unexpected %?", x)),
        }
      },
      None => fail!("decode_hdr_chunked_ok: decoder did not finish"),
    }
  };
}

pub fn decode_body_chunked_ok(chunk_len: uint, bytes: &[u8]) -> ~[u8] {
  let mut decoder = body_decoder::BodyDecoder::new();
  let mut out: ~[u8] = ~[];

  let mut iter = bytes.chunk_iter(chunk_len);
  loop {
    match iter.next() {
      Some(chunk) => {
        let (result, new_out) = decoder.input(chunk, out);
        out = new_out;
        match result {
          Left(new_decoder) => { decoder = new_decoder },
          Right((Ok(()), [])) => { return out },
          x => fail!(fmt!("decode_hdr_chunked_ok: unexpected %?", x)),
        }
      },
      None => fail!("decode_hdr_chunked_ok: decoder did not finish"),
    }
  };
}
