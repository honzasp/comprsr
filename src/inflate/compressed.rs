pub enum BlockPhase {
  LitlenPhase,
  LenExtraPhase(uint,uint), /* (base_len,extra_bits) */
  DistPhase(uint), /* (len) */
  DistExtraPhase(uint,uint,uint), /* (len,base_dist,extra_bits) */
}
