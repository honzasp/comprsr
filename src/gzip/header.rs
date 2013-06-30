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
  pub fn empty() -> Header {
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
