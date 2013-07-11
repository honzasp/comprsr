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
