use inflate::error;

pub enum BlockPhase {
  LitlenPhase,
  LenExtraPhase(uint,uint), /* (base_len,extra_bits) */
  DistPhase(uint), /* (len) */
  DistExtraPhase(uint,uint,uint), /* (len,base_dist,extra_bits) */
}

pub enum LitlenCode {
  LiteralCode(u8),
  LengthCode(uint,uint),
  BlockEndCode,
}

pub fn decode_litlen(code: u16) -> Result<LitlenCode,~error::Error> {
  fail!()
}

pub fn decode_dist(code: u16) -> Result<(uint,uint),~error::Error> {
  fail!()
}
