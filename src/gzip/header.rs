use checksums::crc32;

#[deriving(Eq, Clone)]
pub struct Header {
  is_text: bool,
  has_crc: bool,
  extras: Option<~[Extra]>,
  file_name: Option<~str>,
  comment: Option<~str>,
  extra_flags: u8,
  system: Option<System>,
  mtime: Option<u32>, // TODO: use some date type
}

#[deriving(Eq, Clone)]
pub struct Extra {
  id: (u8, u8),
  data: ~[u8],
}

#[deriving(Eq, Clone)]
pub enum System {
  FAT(),
  Amiga(),
  VMS(),
  Unix(),
  VM_CMS(),
  AtariTOS(),
  HPFS(),
  Macintosh(),
  ZSystem(),
  CP_M(),
  TOPS_20(),
  NTFS(),
  QDOS(),
  AcornRISCOS(),
  Undefined(u8),
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

  pub fn crc32(&self) -> crc32::Crc32 {
    let mut crc = crc32::Crc32::new();
    let flg = 
        if self.is_text             { 0b1 } else {0}
      | if self.has_crc             { 0b10 } else {0}
      | if self.extras.is_some()    { 0b100 } else {0}
      | if self.file_name.is_some() { 0b1000 } else {0}
      | if self.comment.is_some()   { 0b1_0000 } else {0} ;

    crc = crc.update(&[0x1f, 0x8b, 8, flg]);
    crc = crc.update(match self.mtime {
      Some(mtime) => &[
          (mtime      ) as u8,
          (mtime >> 8 ) as u8,
          (mtime >> 16) as u8,
          (mtime >> 24) as u8,
        ],
      None => &[0, 0, 0, 0],
    });
    crc = crc.update(&[self.extra_flags, System::to_number(self.system)]);

    match self.extras {
      None => { },
      Some(ref extras) => {
        let xlen = do extras.iter().fold(0) 
          |xlen, extra| { xlen + 4 + extra.data.len() };
        crc = crc.update(&[xlen as u8, (xlen >> 8) as u8]);

        for extras.iter().advance |extra| {
          let (id1, id2) = extra.id;
          let len = extra.data.len();
          crc = crc.update(&[id1, id2]);
          crc = crc.update(&[len as u8, (len >> 8) as u8]);
          crc = crc.update(extra.data);
        }
      },
    };

    match self.file_name {
      None => { },
      Some(ref file_name) => {
        crc = crc.update(file_name.as_bytes());
        crc = crc.update(&[0]);
      },
    };

    match self.comment {
      None => { },
      Some(ref comment) => {
        crc = crc.update(comment.as_bytes());
        crc = crc.update(&[0]);
      },
    };

    crc
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
      255 => None,
      other => Some(Undefined(other)),
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
        Undefined(x) => x,
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
      Undefined(x) => fmt!("Undefined system %u", x as uint),
    }
  }
}
