#[cfg(test)];
pub use inflate::inflater;
pub use inflate::error;

pub fn inflate_ok(bytes: &[u8]) -> ~[u8] {
  let buf = ~[];
  let mut inflater = inflater::Inflater::new(~buf);
  match inflater.input(bytes) {
    inflater::FinishedRes(rest) if rest.is_empty() => *inflater.close(),
    x => fail!(fmt!("inflate_ok: unexpected Res %?", x)),
  }
}

pub fn inflate_err<'a>(bytes: &'a [u8]) -> (~error::Error, &'a [u8]) {
  let receiver = ();
  let mut inflater = inflater::Inflater::new(~receiver);
  match inflater.input(bytes) {
    inflater::ErrorRes(err,rest) => (err, rest),
    x => fail!(fmt!("inflate_err: unexpected Res %?", x)),
  }
}
