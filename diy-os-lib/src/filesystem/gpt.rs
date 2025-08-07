// reference docs at https://web.archive.org/web/20250610025442/https://uefi.org/specs/UEFI/2.10/05_GUID_Partition_Table_Format.html
// and https://web.archive.org/web/20250630111613/https://wiki.osdev.org/GPT#Layout and https://web.archive.org/web/20250306133759/http://wiki.osdev.org/Partition_Table
// wayback machine links incase any content changes or disappears
use core::ffi::{CStr, c_char};
use core::fmt::Debug;
use core::hint::assert_unchecked;
use core::mem::Assume;
use core::str::FromStr;
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
    pub partion_entry_lba: u64,
    pub num_of_partions: u32,
    pub size_of_partion_entry: u32,
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
        let bytes = unsafe { <[u8; size_of::<Self>()] as TransmuteFrom<Self>>::transmute(copy) };
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
        let ptr_to_start: *const u8 = core::ptr::without_provenance(
            usize::try_from(addr_of_memmaped_drive + (self.partion_entry_lba * 512)).unwrap(),
        );
        let bytes = core::ptr::slice_from_raw_parts(
            ptr_to_start,
            usize::try_from(self.size_of_partion_entry * self.num_of_partions).unwrap(),
        );
        let bytes = unsafe { bytes.as_ref().unwrap() };

        let hash = crc32fast::hash(bytes);

        checksum == hash
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u128)]
#[non_exhaustive]
pub enum FSGuid {
    SimpleFileSystem = 0x5346_5353_4653_061A_450C_11BF_4EBF_0E06,
}

#[derive(Debug)]
#[repr(C, packed)]
pub struct GuidNode(u32, u16);

#[derive(Debug)]
/// String must have exactly 36 characters
// try to also implement the that all chars must be hex
// and needing 4 -
pub struct GuidStr<'a> {
    string: &'a str,
}

impl<'a> const refine::Refined for GuidStr<'a> {
    type Input = &'a str;

    fn new(input: Self::Input) -> Self {
        Self { string: input }
    }

    fn holds(input: &Self::Input) -> bool {
        input.len() == 36
    }
}

impl<'a> From<GuidStr<'a>> for &'a str {
    fn from(value: GuidStr<'a>) -> Self {
        value.string
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct Guid {
    time_low: u32,
    time_mid: u16,
    time_high_and_version: u16,
    clock_seq_high_and_reserved: u8,
    clock_seq_low: u8,
    node: GuidNode,
}

#[derive(thiserror::Error, Debug)]
pub enum ParseGuidError {
    #[error("Faild to find 4 - to split the string at")]
    FailedToSplitStrings,
    #[error("The time low part of str was too short")]
    TimeLowTooShort,
    #[error("The time mid part of str was too short")]
    TimeMidTooShort,
    #[error("The time high part of str was too short")]
    TimeHighTooShort,
    #[error("The Clock Seq of str was too short")]
    ClockSeqTooShort,
    #[error("The node part of str was too short")]
    NodeTooShort,
}

impl Guid {
    pub const fn from_u128(value: u128) -> Self {
        unsafe { core::mem::transmute::<u128, Self>(value) }
    }

    fn from_guid_str(str: &GuidStr) -> Result<Self, ParseGuidError> {
        let str = str.string;

        // Safe since we know that guild string must have a len of 36
        unsafe { assert_unchecked(str.len() == 36) };

        // should add a check for number of - and hex instead of unwrapping

        let mut substrings = str.split('-');
        let time_low_str = substrings
            .next()
            .ok_or(ParseGuidError::FailedToSplitStrings)?;
        // in the future test weather to see if std lib or my impl is faster
        // from_ascii_radix is the stdlibs
        // nvm there is 100 faster sin
        // keeping mine cause i want to for now
        let time_low = time_low_str
            .bytes()
            .array_chunks::<2>()
            .map(|pair| u8::from_ascii_radix(&pair, 16).unwrap())
            .rev()
            .array_chunks::<4>()
            .map(|four_bytes| unsafe { <u32 as TransmuteFrom<[u8; 4]>>::transmute(four_bytes) })
            .next()
            .ok_or(ParseGuidError::TimeLowTooShort)?;

        let time_mid_str = substrings
            .next()
            .ok_or(ParseGuidError::FailedToSplitStrings)?;

        let time_mid = time_mid_str
            .bytes()
            .array_chunks::<2>()
            .map(|pair| u8::from_ascii_radix(&pair, 16).unwrap())
            .rev()
            .array_chunks::<2>()
            .map(|two_bytes| unsafe { <u16 as TransmuteFrom<[u8; 2]>>::transmute(two_bytes) })
            .next()
            .ok_or(ParseGuidError::TimeMidTooShort)?;

        let time_high_and_version_str = substrings
            .next()
            .ok_or(ParseGuidError::FailedToSplitStrings)?;

        let time_high_and_version = time_high_and_version_str
            .bytes()
            .array_chunks::<2>()
            .map(|pair| u8::from_ascii_radix(&pair, 16).unwrap())
            .rev()
            .array_chunks::<2>()
            .map(|two_bytes| unsafe { <u16 as TransmuteFrom<[u8; 2]>>::transmute(two_bytes) })
            .next()
            .ok_or(ParseGuidError::TimeHighTooShort)?;

        let clock_seq_high_and_low_str = substrings
            .next()
            .ok_or(ParseGuidError::FailedToSplitStrings)?;

        let clock_seq_high_and_low = clock_seq_high_and_low_str
            .bytes()
            .array_chunks::<2>()
            .map(|pair| u8::from_ascii_radix(&pair, 16).unwrap())
            .array_chunks::<2>()
            .next()
            .ok_or(ParseGuidError::ClockSeqTooShort)?;

        let node_str = substrings
            .next()
            .ok_or(ParseGuidError::FailedToSplitStrings)?;

        let node = node_str
            .bytes()
            .array_chunks::<2>()
            .map(|pair| u8::from_ascii_radix(&pair, 16).unwrap())
            .array_chunks::<6>()
            .map(|two_bytes| unsafe {
                <GuidNode as TransmuteFrom<[u8; 6], { Assume::SAFETY }>>::transmute(two_bytes)
            })
            .next()
            .ok_or(ParseGuidError::NodeTooShort)?;

        Ok(Self {
            time_low,
            time_mid,
            time_high_and_version,
            clock_seq_high_and_reserved: clock_seq_high_and_low[0],
            clock_seq_low: clock_seq_high_and_low[1],
            node,
        })
    }
}

impl const From<Guid> for u128 {
    fn from(value: Guid) -> Self {
        unsafe { core::mem::transmute::<Guid, Self>(value) }
    }
}

impl const From<u128> for Guid {
    fn from(value: u128) -> Self {
        Self::from_u128(value)
    }
}

impl TryFrom<&str> for Guid {
    type Error = ParseGuidError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::from_str(value)
    }
}

impl FromStr for Guid {
    type Err = ParseGuidError;

    fn from_str(str: &str) -> Result<Self, Self::Err> {
        use refine::Refined;

        // inline marco to make generics work, patch lib with this support
        let guid_str = {
            type Input<'a> = <GuidStr<'a> as Refined>::Input;
            // add to lib cfg if input == to same time as input to marco to not
            // gen the into
            let value: Input = str;
            if <GuidStr>::holds(&value) {
                <GuidStr>::new(value)
            } else {
                panic!("predicate does not hold at run time");
            }
        };

        Self::from_guid_str(&guid_str)
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
