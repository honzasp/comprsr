#[cfg(test)];
pub use inflate::inflater;
pub use inflate::error;

pub fn inflate_ok(bytes: &[u8]) -> ~[u8] {
  let mut buf = ~[];
  let mut inflater = do inflater::Inflater::new |chunk| {
    buf.push_all(chunk);
  };

  match inflater.input(bytes) {
    inflater::FinishedRes(rest) if rest.is_empty() => buf,
    x => fail!(fmt!("inflate_ok: unexpected Res %?", x)),
  }
}

pub fn inflate_err(bytes: &[u8]) -> ~error::Error {
  let mut inflater = do inflater::Inflater::new |_| { };
  match inflater.input(bytes) {
    inflater::ErrorRes(err,rest) =>
      if rest.is_empty() {
        err
      } else {
        fail!(fmt!("inflate_err: got err %?, rest %?", err, rest));
      },
    x => fail!(fmt!("inflate_err: unexpected Res %?", x)),
  }
}
