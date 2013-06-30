use inflate;

#[deriving(Clone,Eq)]
pub enum Error {
  InflateError(~inflate::error::Error),
}

impl ToStr for Error {
  pub fn to_str(&self) -> ~str {
    match *self {
      InflateError(ref err) =>
        fmt!("Inflate error: %s", err.to_str()),
    }
  }
}
