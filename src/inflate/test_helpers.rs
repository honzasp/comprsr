#[cfg(test)];
pub use inflate::inflater;
pub use inflate::error;

pub fn inflate_ok(bytes: &[u8]) -> ~[u8] {
  let inflater = inflater::Inflater::new();
  match inflater.input(bytes, ~[]) {
    (Right((Ok(()), [])), inflated) => inflated,
    other => fail!(fmt!("inflate_ok: unexpected Res %?", other)),
  }
}

pub fn inflate_err<'a>(bytes: &'a [u8]) -> (~error::Error, &'a [u8]) {
  let inflater = inflater::Inflater::new();
  match inflater.input(bytes, ()) {
    (Right((Err(error), rest)), ()) => (error, rest),
    other => fail!(fmt!("inflate_err: unexpected Res %?", other)),
  }
}

pub fn inflate_chunked_ok(chunk_len: uint, bytes: &[u8]) -> ~[u8] {
  let mut inflater = inflater::Inflater::new();
  let mut out: ~[u8] = ~[];

  let mut iter = bytes.chunk_iter(chunk_len);
  loop {
    match iter.next() {
      Some(chunk) => {
        let (result, new_out) = inflater.input(chunk, out);
        out = new_out;
        match result {
          Left(new_inflater) => { inflater = new_inflater },
          Right((Ok(()), [])) => { return out; },
          other => fail!(fmt!("inflate_chunked_ok: unexpected %?", other)),
        }
      },
      None => fail!("inflate_chunked_ok: inflater did not finish"),
    }
  }
}


