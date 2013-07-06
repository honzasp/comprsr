#[deriving(Eq, Clone)]
pub struct Header {
  is_text: bool,
  has_crc: bool,
  extras: Option<~[Extra]>,
  file_name: Option<~str>,
  comment: Option<~str>,
  extra_flags: u8,
  system: Option<System>,
  mtime: Option<u32>,
}

#[deriving(Eq, Clone)]
pub struct Extra {
  id: (u8, u8),
  data: ~[u8],
}

#[deriving(Eq, Clone)]
pub enum System {
  FAT,
  Amiga,
  VMS,
  Unix,
  VM_CMS,
  AtariTOS,
  HPFS,
  Macintosh,
  ZSystem,
  CP_M,
  TOPS_20,
  NTFS,
  QDOS,
  AcornRISCOS
}

impl Header {
  pub fn blank() -> Header {
    Header {
      is_text: false,
      has_crc: false,
      extras: None,
      file_name: None,
      comment: None,
      extra_flags: 0,
      system: None,
      mtime: None
    }
  }
}

impl System {
  pub fn from_number(num: u8) -> Option<System> {
    match num {
      0 => Some(FAT),
      1 => Some(Amiga),
      2 => Some(VMS),
      3 => Some(Unix),
      4 => Some(VM_CMS),
      5 => Some(AtariTOS),
      6 => Some(HPFS),
      7 => Some(Macintosh),
      8 => Some(ZSystem),
      9 => Some(CP_M),
      10 => Some(TOPS_20),
      11 => Some(NTFS),
      12 => Some(QDOS),
      13 => Some(AcornRISCOS),
      _ => None
    }
  }

  pub fn to_number(opt_sys: Option<System>) -> u8 {
    match opt_sys {
      Some(sys) => match sys {
        FAT => 0,
        Amiga => 1,
        VMS => 2,
        Unix => 3,
        VM_CMS => 4,
        AtariTOS => 5,
        HPFS => 6,
        Macintosh => 7,
        ZSystem => 8,
        CP_M => 9,
        TOPS_20 => 10,
        NTFS => 11,
        QDOS => 12,
        AcornRISCOS => 13,
      },
      None => 255
    }
  }

  pub fn to_str(opt_sys: Option<System>) -> ~str {
    match opt_sys {
      Some(sys) => sys.to_str(),
      None => ~"unknown",
    }
  }
}

impl ToStr for System {
  fn to_str(&self) -> ~str {
    match *self {
      FAT => ~"FAT filesystem (MS-DOS, OS/2, NT/Win32)",
      Amiga => ~"Amiga",
      VMS => ~"VMS (or OpenVMS)",
      Unix => ~"Unix",
      VM_CMS => ~"VM/CMS",
      AtariTOS => ~"Atari TOS",
      HPFS => ~"HPFS filesystem (OS/2, NT)",
      Macintosh => ~"Macintosh",
      ZSystem => ~"Z-System",
      CP_M => ~"CP/M",
      TOPS_20 => ~"TOPS-20",
      NTFS => ~"NTFS filesystem (NT)",
      QDOS => ~"QDOS",
      AcornRISCOS => ~"Acorn RISCOS",
    }
  }
}

#[cfg(test)]
mod test {
  use gzip::test_helpers::*;
  use gzip::header;
  use gzip::error;

  fn header(f: &fn(&mut header::Header)) -> ~header::Header {
    let mut header = ~header::Header::blank();
    f(header);
    header
  }

  #[test]
  fn test_decode_header_ok() {
    { // blank header and empty body
      assert_eq!(decode_ok1(&[
          0x1f, 0x8b, // magic
          8, 0b000_00000, // cm, flags
          0, 0, 0, 0, // mtime
          0, 255, // extra flags, system
          // no compressed data
          0, 0, 0, 0, // crc32
          0, 0, 0, 0, // input size
        ]),
        (~header::Header::blank(), ~[])
      );
    }

    { // set mtime, system and extra flags
      let hdr = do header |h| {
        h.extra_flags = 42;
        h.mtime = Some(0x87654321);
        h.system = Some(header::Amiga);
      };

      assert_eq!(decode_ok1(&[
          0x1f, 0x8b, 0x08, 0x00,
          0x21, 0x43, 0x65, 0x87,
          0x00, 0x01,
          0xe3, 0x12, 0x91, 0x03, 0x00a,
          0xf2, 0xb6, 0x77, 0x26,
          0x03, 0x00, 0x00, 0x00
        ]),
        (hdr, ~[10, 20, 30])
      );
    }
  }

  #[test]
  fn test_decode_header_err() {
    { // bad magic number
      assert_eq!(decode_err(&[
          0x1f, 0x8a, 2, 3, 5, 7
        ]),
        (~error::BadMagicNumber(0x8b_1f, 0x8a_1f), Some(&[2, 3, 5, 7]))
      );
    }

    { // bad compression method
      assert_eq!(decode_err(&[
          0x1f, 0x8b, 77, 2, 3, 5, 7
        ]),
        (~error::BadCompressionMethod(77), Some(&[2, 3, 5, 7]))
      );
    }

    { // reserved flag set on
      assert_eq!(decode_err(&[
          0x1f, 0x8b, 8, 0b010_00000, 2, 3, 5, 7
        ]),
        (~error::ReservedFlagUsed(6), Some(&[2, 3, 5, 7]))
      );
    }

    { // unterminated header
      assert_eq!(decode_err(&[
          0x1f, 0x8b, 8
        ]),
        (~error::UnterminatedHeader(10, 3), None)
      );
    }
  }

  #[test]
  fn test_decode_header_extras() {
    { // none extra field
      let hdr = do header |h| {
        h.extras = Some(~[]);
      };

      assert_eq!(decode_ok1(&[
          0x1f, 0x8b, 8, 0b000_00100,
          0, 0, 0, 0, 0, 255, 
          0, 0, 
          0, 0, 0, 0, 0, 0, 0, 0, 
        ]),
        (hdr, ~[])
      );
    }

    { // two small extra fields
      let hdr = do header |h| {
        h.extras = Some(~[
            header::Extra {
              id: (11, 22),
              data: ~[110, 120, 130],
            },
            header::Extra {
              id: (44, 2),
              data: ~[2, 3, 5, 7, 11],
            }
          ]);
      };

      assert_eq!(decode_ok1(&[
          0x1f, 0x8b, 8, 0b000_00100,
          0, 0, 0, 0, 0, 255, 
          14, 0, 
            11, 22, 3, 110, 120, 130,
            44, 2, 5, 2, 3, 5, 7, 11,
          0, 0, 0, 0, 0, 0, 0, 0, 
        ]),
        (hdr, ~[])
      );
    }

    { // end in the middle of a field
      assert_eq!(decode_err(&[
          0x1f, 0x8b, 8, 0b000_00100,
          0, 0, 0, 0, 0, 255, 
          14, 0, 
            11, 22, 3, 110, 
        ]),
        (~error::UnterminatedExtra(14, 4), None)
      );
    }

    { // end after a complete field
      assert_eq!(decode_err(&[
          0x1f, 0x8b, 8, 0b000_00100,
          0, 0, 0, 0, 0, 255, 
          14, 0, 
            11, 22, 3, 110, 120, 130,
        ]),
        (~error::UnterminatedExtra(14, 6), None)
      );
    }
  }

  #[test]
  fn test_decode_header_file_name() {
    {
      let hdr = do header |h| {
        h.file_name = Some(~"decoder.rs");
      };

      assert_eq!(decode_ok1(&[
          0x1f, 0x8b, 8, 0b000_01000,
          0, 0, 0, 0, 0, 255, 
            100, 101, 99, 111, 100, 101, 114, 46, 114, 115, 0,
          0, 0, 0, 0, 0, 0, 0, 0, 
        ]),
        (hdr, ~[])
      );
    }

    { // unterminated file name
      assert_eq!(decode_err(&[
          0x1f, 0x8b, 8, 0b000_01000,
          0, 0, 0, 0, 0, 255, 
            100, 101, 99, 111, 100,
        ]),
        (~error::UnterminatedFileName(5), None)
      );
    }
  }

  #[test]
  fn test_decode_header_comment() {
    {
      let hdr = do header |h| {
        h.file_name = Some(~"Created by comprsr");
      };

      assert_eq!(decode_ok1(&[
          0x1f, 0x8b, 8, 0b000_10000,
          0, 0, 0, 0, 0, 255, 
            67, 114, 101, 97, 116, 101, 100, 32, 98,
            121, 32, 99, 111, 109, 112, 114, 115, 114, 0,
          0, 0, 0, 0, 0, 0, 0, 0, 
        ]),
        (hdr, ~[])
      );
    }

    { // unterminated comment
      assert_eq!(decode_err(&[
          0x1f, 0x8b, 8, 0b000_10000,
          0, 0, 0, 0, 0, 255, 
            67, 114, 101, 97, 116, 101, 100, 32, 98,
        ]),
        (~error::UnterminatedComment(9), None)
      );
    }
  }

  #[test]
  fn test_decode_header_crc() {
    { // CRC is ok
      let hdr = do header |h| {
        h.mtime = Some(0xdead_beef);
        h.extras = Some(~[
            header::Extra { id: (2, 3), data: ~[1, 2, 3, 5, 8], }
          ]);
        h.comment = Some(~"Fibonacci");
        h.has_crc = true;
      };

      assert_eq!(decode_ok1(&[
          0x1f, 0x8b, 8, 0b000_10110,
          0xef, 0xbe, 0xad, 0xde,
          0, 255,
          7, 0,   2, 3, 1, 2, 3, 5, 8,
          70, 105, 98, 111, 110, 97, 99, 99, 105, 0,
          0xc0, 0x71,
          0, 0, 0, 0, 0, 0, 0, 0
        ]),
        (hdr, ~[])
      );
    }

    { // CRC is wrong
      assert_eq!(decode_err(&[
          0x1f, 0x8b, 8, 0b000_10110,
          0xef, 0xbe, 0xad, 0xde,
          0, 255,
          7, 0,   2, 3, 1, 2, 3, 5, 8,
          70, 105, 98, 111, 110, 97, 99, 99, 105, 0,
          0xc0, 0xdd,
          2, 3, 4, 5,
        ]),
        (~error::BadHeaderChecksum(0x71c0, 0xddc0), Some(&[2, 3, 4, 5]))
      );
    }

    { // CRC is unterminated
      assert_eq!(decode_err(&[
          0x1f, 0x8b, 8, 0b000_00010,
          0xef, 0xbe, 0xad, 0xde,
          0, 255,
          0x12
        ]),
        (~error::UnterminatedHeaderChecksum(2, 1), None)
      );
    }
  }
}
