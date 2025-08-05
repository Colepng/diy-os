use core::ascii::Char;

pub struct PartionTableHeader {
    pub signature: Signature,
}

pub struct Signature {
    chars: [Char; 8],
}

impl Signature {
    const VALID: Self = Self {
        chars: [
            Char::CapitalE,
            Char::CapitalF,
            Char::CapitalI,
            Char::Space,
            Char::CapitalP,
            Char::CapitalA,
            Char::CapitalR,
            Char::CapitalT,
        ],
    };

    pub fn valid(&self) -> bool {
        self.chars == Self::VALID.chars
    }
}

pub mod mbr {
    #[repr(C, packed)]
    pub struct MBR {
        boot_code: [u8; 440],
        unique_mbr_disk_signature: u32,
        unknown: u16,
        pub partion_record: [PartionTableEntry; 4],
        pub signature: u16,
    }

    #[derive(Debug, Clone, Copy)]
    #[repr(C)]
    pub struct PartionTableEntry {
        drive_attributes: u8,
        starting_chs: CHSAddress,
        os_type: u8,
        ending_chs: CHSAddress,
        starting_lba: u32,
        ending_lba: u32,
    }

    #[derive(Debug, Clone, Copy)]
    #[allow(dead_code)]
    pub struct CHSAddress(u8, u8, u8);
}
