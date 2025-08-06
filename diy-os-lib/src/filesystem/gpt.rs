// reference docs at https://web.archive.org/web/20250610025442/https://uefi.org/specs/UEFI/2.10/05_GUID_Partition_Table_Format.html
// and https://web.archive.org/web/20250630111613/https://wiki.osdev.org/GPT#Layout and https://web.archive.org/web/20250306133759/http://wiki.osdev.org/Partition_Table
// wayback machine links incase any content changes or disappears
use core::ffi::{CStr, c_char};
use core::fmt::Debug;
use core::{ascii::Char, mem::TransmuteFrom};

#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
#[repr(C, packed)]
// should add ending reserved fields
pub struct PartionTableHeader {
    pub signature: Signature,
    gpt_revison: u32,
    header_size: u32,
    crc32_checksum: u32,
    reserved: u32,
    lba_table_header: u64,
    lba_alt_table_header: u64,
    first_usable_logical_block: u64,
    last_usable_logical_block: u64,
    disk_guid: u128,
    partion_entry_lba: u64,
    num_of_partions: u32,
    size_of_partion_entry: u32,
    crc32_partion_entry_array: u32,
}

impl Debug for PartionTableHeader {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let gpt_revison = self.gpt_revison;
        let header_size = self.header_size;
        let crc32_checksum = self.crc32_checksum;
        let reserved = self.reserved;
        let lba_table_header = self.lba_table_header;
        let lba_alt_table_header = self.lba_alt_table_header;
        let first_usable_logical_block = self.first_usable_logical_block;
        let last_usable_logical_block = self.last_usable_logical_block;
        let disk_guid = self.disk_guid;
        let partion_entry_lba = self.partion_entry_lba;
        let num_of_partions = self.num_of_partions;
        let size_of_partion_entry = self.size_of_partion_entry;
        let crc32_partion_entry_array = self.crc32_partion_entry_array;
        f.debug_struct("Partion Table Header")
            .field("signature", &self.signature)
            .field("gpt_revison", &gpt_revison)
            .field("header_size", &header_size)
            .field("crc32_checksum", &crc32_checksum)
            .field("reserved", &reserved)
            .field("lba_table_header", &lba_table_header)
            .field("lba_alt_table_header", &lba_alt_table_header)
            .field("first_usable_logical_block", &first_usable_logical_block)
            .field("last_usable_logical_block", &last_usable_logical_block)
            .field("disk_guid", &disk_guid)
            .field("partion_entry_lba", &partion_entry_lba)
            .field("num_of_partions", &num_of_partions)
            .field("size_of_partion_entry", &size_of_partion_entry)
            .field("crc32_partion_entry_array", &crc32_partion_entry_array)
            .finish()
    }
}

impl PartionTableHeader {
    pub fn validate(&self, addr: u64) -> bool {
        let crc32 = self.valid_crc32_checksum();
        let sig = self.signature.valid();
        let crc32_partions = self.valid_partion_array(addr);

        if self.lba_table_header == 1 {
            crc32 && sig && crc32_partions && self.valid_self_lba() && self.valid_last_lba(addr)
        } else {
            crc32 && sig && crc32_partions
        }
    }

    fn valid_crc32_checksum(&self) -> bool {
        let mut copy = *self;
        let checksum = self.crc32_checksum;

        copy.crc32_checksum = 0;
        // Safe to transmute since the transmute from validates all conditions
        let bytes = unsafe {
            <[u8; size_of::<Self>()] as TransmuteFrom<Self>>::transmute(
                copy,
            )
        };
        let hash = crc32fast::hash(&bytes);

        hash == checksum
    }

    /// Actually implement this, rn assuming it's in lba 1
    const fn valid_self_lba(&self) -> bool {
        self.lba_table_header == 1
    }

    fn valid_last_lba(&self, addr_of_memmaped_drive: u64) -> bool {
        let ptr_to_alt: *const Self = core::ptr::without_provenance(
            // assuming block/sector size is 512 bytes
            usize::try_from(addr_of_memmaped_drive + (512 * self.lba_alt_table_header)).unwrap(),
        );

        let alt_header = unsafe { *ptr_to_alt };

        alt_header.validate(addr_of_memmaped_drive)
    }

    fn valid_partion_array(&self, addr_of_memmaped_drive: u64) -> bool {
        let checksum = self.crc32_partion_entry_array;

        // assuming block/sector size is 512 bytes
        let ptr_to_start: *const u8 = core::ptr::without_provenance(usize::try_from(addr_of_memmaped_drive + (self.partion_entry_lba * 512)).unwrap());
        let bytes = core::ptr::slice_from_raw_parts(ptr_to_start, usize::try_from(self.size_of_partion_entry * self.num_of_partions).unwrap());
        let bytes = unsafe { bytes.as_ref().unwrap() };

        let hash = crc32fast::hash(bytes);

        checksum == hash
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
// should add ending reserved fields
pub struct PartitionEntry {
    partion_type_guid: u128,
    unique_partion_guid: u128,
    starting_lba: u64,
    ending_lba: u64,
    attributes: u64,
    pub partion_name: [c_char; 72],
}

impl PartitionEntry {
    pub fn name(&self) -> &'static str {
        let ptr = self.partion_name.as_ptr();
        let cstr = unsafe { CStr::from_ptr(ptr) };
        cstr.to_str().unwrap()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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
