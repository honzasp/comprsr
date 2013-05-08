use deflate::bit_reader::{BitReader};
use deflate::error::*;

#[path = "decompress/non_compressed.rs"] mod non_compressed;
#[path = "decompress/compressed.rs"]     mod compressed;
#[path = "decompress/fixed.rs"]          mod fixed;
#[path = "decompress/dynamic.rs"]        mod dynamic;

pub fn decompress(in: &mut BitReader) -> Result<~[u8],~DeflateError> {
  use deflate::decompress::non_compressed::non_compressed_block;
  use deflate::decompress::dynamic::dynamic_compressed_block;
  use deflate::decompress::fixed::fixed_compressed_block;

  let mut out: ~[u8] = ~[];

  loop {
    let bfinal = in.read_bit();
    let btype = in.read_bits(2);

    match btype {
      0b00 => match non_compressed_block(in, &mut out) {
          Some(err) => return Err(err), _ => { }
        },
      0b01 => match fixed_compressed_block(in, &mut out) {
          Some(err) => return Err(err), _ => { }
        },
      0b10 => match dynamic_compressed_block(in, &mut out) {
          Some(err) => return Err(err), _ => { }
        },
      _ => return Err(~BadBlockType)
    }

    if bfinal != 0 {
      break
    } else if in.eof() {
      return Err(~UnexpectedEOFError)
    }
  }
  
  Ok(out)
}

#[cfg(test)]

mod test {
  use deflate::decompress::{decompress};
  use deflate::bit_reader::{BitReader};
  use deflate::error::*;

  #[test]
  fn test_decompress_non_compressed() {
    /* one block */
    let mut reader1 = BitReader::new(~[
        0b00000_001,
        0b00001010, 0b00000000,
        0b11110101, 0b11111111,
        10, 20, 30, 40, 50, 60, 70, 80, 90, 100
      ]);

    assert_eq!(decompress(reader1).unwrap(), ~[
        10, 20, 30, 40, 50, 60, 70, 80, 90, 100]);

    /* two blocks */
    let mut reader2 = BitReader::new(~[
        0b00000_000,
        0b0000_0110, 0b0000_0000,
        0b1111_1001, 0b1111_1111,
        11, 22, 33, 44, 55, 66,
        0b00000_001,
        0b0000_0100, 0b0000_0000,
        0b1111_1011, 0b1111_1111,
        77, 88, 99, 110
      ]);

    assert_eq!(decompress(reader2).unwrap(), ~[
      11, 22, 33, 44, 55, 66, 77, 88, 99, 110]);
  }

  #[test]
  fn test_decompress_non_compressed_errors() {
    /* the length and the inverse don't match */
    let mut reader1 = BitReader::new(~[
      0b00000_001,
      0b0000_0101, 0b0000_0000,
      0b1100_0000, 0b1111_1111]);

    match decompress(reader1) {
      Err(~LengthMismatchError(0b0000_0000_0000_0101, 0b1111_1111_1100_0000)) =>
        { /* ok */ },
      Err(err) => fail!(fmt!("unexpected error %s", err.to_str())),
      _ => fail!(~"expected an error")
    }

    /* the stream is too short */
    let mut reader2 = BitReader::new(~[
        0b00000_000,
        0b00001010, 0b00000000,
        0b11110101, 0b11111111,
        10, 20, 30, 40 /* and 6 missing bytes */
      ]);

    match decompress(reader2) {
      Err(~UnexpectedEOFError) => { /* ok */ },
      Err(err) => fail!(fmt!("unexpected error %s", err.to_str())),
      _ => fail!(~"expected an error")
    }
  }

  #[test]
  fn test_decompress_fixed() {
    /* literals only */
    let mut reader1 = BitReader::new(~[
      0b11100011, 0b00010010, 0b10010001, 0b00000011, 0b00000000]);

    assert_eq!(decompress(reader1).unwrap(), ~[10, 20, 30]);

    /* longer data */
    let mut reader2 = BitReader::new(~[
      0b11111011, 0b10110001, 0b01101010, 0b01101001,
      0b11111010, 0b10110111, 0b01111111, 0b01011110,
      0b11001101, 0b10111011, 0b10101011, 0b00000001]);

    assert_eq!(decompress(reader2).unwrap(), ~[
      248, 170, 165, 103, 246, 254, 74, 131, 187, 123]);

    /* simple length-distance pair */
    let mut reader3 = BitReader::new(~[
      0b10010011, 0b11010011, 0b00000010,
      0b00000010, 0b00001101, 0b00000000]);

    assert_eq!(decompress(reader3).unwrap(), ~[
      30, 42, 42, 42, 42, 40]);

    /* a longer length-distance pair */
    let mut reader4 = BitReader::new(~[
      0b11100011, 0b00010010, 0b01010011,
      0b11000100, 0b00001101, 0b10111001,
      0b11000100, 0b00010100, 0b00000001]);

    assert_eq!(decompress(reader4).unwrap(), ~[
      10, 22, 33, 22, 33, 22, 33, 22, 33, 22, 33, 22, 33, 22, 33, 22, 33, 22,
      33, 22, 33, 22, 33, 22, 33, 22, 33, 22, 33, 10, 22, 33]);

    /* long repetition and then long distance */
    let mut reader5 = BitReader::new(~[
      0b00010011, 0b10010001, 0b11010011, 0b00110000, 0b10110010,
      0b11100001, 0b00011010, 0b00000101, 0b00100011, 0b00001110,
      0b10001000, 0b10000000, 0b00100010, 0b00011110, 0b00000000]);

    let bytes5 = decompress(reader5).unwrap();

    assert_eq!(bytes5.slice(0, 5), &[20,30,40,50,60]);
    for uint::range(5, 505) |i| {
      assert_eq!(bytes5[i], 10);
    }
    assert_eq!(bytes5.slice(505, 510), &[20,30,40,50,60]);
  }

  #[test]
  fn test_decompress_fixed_errors() {
    /* the distance is too long (points before the start of input) */
    let mut reader1 = BitReader::new(~[
      0b1110_0011, 0b0001_0010, 0b0000_0011,
      0b0001_0010, 0b0000_0000]);

    match decompress(reader1) {
      Err(~DistanceTooLong(2, 8)) => { /* ok */ },
      Err(err) => fail!(fmt!("got error %s", err.to_str())),
      _ => fail!(~"expected error")
    }
  }

  #[test]
  fn test_decompress_dynamic() {
    let mut reader1 = BitReader::new(~[
      0b00001101, 0b11000101, 0b10110001, 0b00000001, 0b00000000,
      0b00000000, 0b00001000, 0b11000010, 0b10110000, 0b01010010,
      0b11111000, 0b11111111, 0b01100110, 0b11001101, 0b10010010,
      0b10101100, 0b00000001, 0b11011100, 0b10001100, 0b01100010,
      0b11111101, 0b01001001, 0b00001111]);

    assert_eq!(decompress(reader1).unwrap(), ~[
      1,4,3,1,0,0,0,2,4,4,2,1,2,2,0,2,3,2,2,1,2,0,1,3]);
  }

  #[test]
  fn test_decompress_bad_btype() {
    let mut reader1 = BitReader::new(~[0b110]);

    match decompress(reader1) {
      Err(~BadBlockType) => { },
      Err(err) => fail!(fmt!("unexpected %s", err.to_str())),
      Ok(_) => fail!(~"expected an error")
    }
  }
}
