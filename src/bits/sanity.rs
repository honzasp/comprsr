#[macro_escape];

#[cfg(not(no_sanity))]
#[macro_escape]
mod live_sanity {
  macro_rules! sanity(
    ($cond:expr) => (
      if !$cond {
        fail!(fmt!("Sanity check failed: %s", stringify!($cond)));
      }
    )
  )
}

#[cfg(no_sanity)]
#[macro_escape]
mod dead_sanity {
  macro_rules! sanity(
    ($cond:expr) => ( { } )
  )
}
