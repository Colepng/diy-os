// reference docs at https://web.archive.org/web/20250610025442/https://uefi.org/specs/UEFI/2.10/05_GUID_Partition_Table_Format.html
// and https://web.archive.org/web/20250630111613/https://wiki.osdev.org/GPT#Layout and https://web.archive.org/web/20250306133759/http://wiki.osdev.org/Partition_Table
// wayback machine links incase any content changes or disappears
use alloc::sync::Arc;
use core::fmt::Debug;
use core::hint::assert_unchecked;
use core::mem::Assume;
use core::mem::TransmuteFrom;
use core::mem::offset_of;
use core::str::FromStr;
use crc32fast::Hasher;
use zerocopy::little_endian::{U16, U32, U64, U128};
use zerocopy::{FromBytes, Immutable, IntoBytes, Unaligned};
use zerocopy::{FromZeros, KnownLayout};

use alloc::string::String;

use crate::device_manager::{BlockDevice, BlockDeviceError};
use crate::multitasking::mutex::Mutex;

#[derive(Debug, KnownLayout, Immutable, Unaligned, FromBytes, IntoBytes)]
#[repr(C)]
pub struct PartionTableHeaderRaw {
    pub signature: [u8; 8],
    gpt_revison: U32,
    header_size: U32,
    crc32_checksum: U32,
    reserved: U32,
    lba_table_header: U64,
    lba_alt_table_header: U64,
    first_usable_logical_block: U64,
    last_usable_logical_block: U64,
    disk_guid: U128,
    pub partion_entry_lba: U64,
    pub num_of_partions: U32,
    pub size_of_partition_entry: U32,
    crc32_partion_entry_array: U32,
}
#[derive(Debug)]
// should add ending reserved fields
#[non_exhaustive]
pub struct PartionTableHeader {
    pub lba_table_header: u64,
    pub lba_alt_table_header: u64,
    pub first_usable_logical_block: u64,
    pub last_usable_logical_block: u64,
    pub disk_guid: u128,
    pub partion_entry_lba: u64,
    pub num_of_partions: u32,
    pub size_of_partion_entry: u32,
}

#[derive(thiserror::Error, Debug)]
pub enum PartionTableHeaderError {
    #[error("The underlying device ran into an error")]
    DeviceError(#[from] BlockDeviceError),
    #[error("Invalid signature, received")]
    InvalidSignature([u8; 8]),
    #[error("Invalid crc32 header checksum expected: `{expected}` but calculated `{calculated}`")]
    InvalidCrc32HeaderChecksum { expected: u32, calculated: u32 },
    #[error(
        "Invalid crc32 partion entries checksum expected: `{expected}` but calculated `{calculated}`"
    )]
    InvalidCrc32PartionEntriesChecksum { expected: u32, calculated: u32 },
}

impl PartionTableHeader {
    /// Reads and validates the GPT partition table header from `device`.
    ///
    /// Reads LBA 1, checks the `"EFI PART"` signature, validates the header CRC32
    /// (computed with the CRC field zeroed), then reads and validates the partition
    /// entry array CRC32.
    ///
    /// # Errors
    ///
    /// Returns [`PartionTableHeaderError`] if:
    /// - the underlying device read fails
    /// - the GPT signature is missing or wrong
    /// - either the header or partition entry array CRC fails to match
    pub fn from_device(
        device: &Arc<Mutex<dyn BlockDevice>>,
    ) -> Result<Self, PartionTableHeaderError> {
        let mut header = PartionTableHeaderRaw::new_zeroed();

        {
            let mut drive = device.acquire();

            drive.read_sectors(1, 1, header.as_mut_bytes())?;
        }

        if *b"EFI PART" != header.signature {
            return Err(PartionTableHeaderError::InvalidSignature(header.signature));
        }

        // Validate partition header checksum
        let mut hasher = Hasher::new();
        let bytes = header.as_bytes();

        // Relies on the order of field to compute checksum
        hasher.update(&bytes[0..const { offset_of!(PartionTableHeaderRaw, crc32_checksum) }]);
        hasher.update(
            &[0u8; const {
                offset_of!(PartionTableHeaderRaw, reserved)
                    - offset_of!(PartionTableHeaderRaw, crc32_checksum)
            }],
        );
        hasher.update(&bytes[const { offset_of!(PartionTableHeaderRaw, reserved) }..]);

        let hash = hasher.finalize();

        if hash != header.crc32_checksum.get() {
            return Err(PartionTableHeaderError::InvalidCrc32HeaderChecksum {
                expected: header.crc32_checksum.get(),
                calculated: hash,
            });
        }

        // Validate partition entries checksum
        let array_size = usize::try_from(header.size_of_partition_entry.get())
            .unwrap()
            .checked_mul(usize::try_from(header.num_of_partions.get()).unwrap())
            .unwrap();

        let mut buffer = alloc::vec![0u8; array_size];

        {
            let mut drive = device.acquire();

            let sector_size = drive.sector_size();

            drive.read_sectors(
                header.partion_entry_lba.get(),
                u8::try_from(array_size.div_ceil(sector_size)).unwrap(),
                &mut buffer,
            )?;

            let hash = crc32fast::hash(buffer.as_bytes());

            if hash != header.crc32_partion_entry_array.get() {
                return Err(
                    PartionTableHeaderError::InvalidCrc32PartionEntriesChecksum {
                        expected: header.crc32_partion_entry_array.get(),
                        calculated: hash,
                    },
                );
            }
        }

        // TODO: FIx
        // Check if current header is alt or not
        // if header.lba_table_header.get() == 1 {
        //     // If it successfully returns the header must be valid
        //     let _ = Self::from_device(device, true)?;
        // }

        // TODO validate self lba?? lowkey forget what this is been way too long

        Ok(Self {
            lba_table_header: header.lba_table_header.get(),
            lba_alt_table_header: header.lba_alt_table_header.get(),
            first_usable_logical_block: header.first_usable_logical_block.get(),
            last_usable_logical_block: header.last_usable_logical_block.get(),
            disk_guid: header.disk_guid.get(),
            partion_entry_lba: header.partion_entry_lba.get(),
            num_of_partions: header.num_of_partions.get(),
            size_of_partion_entry: header.size_of_partition_entry.get(),
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u128)]
#[non_exhaustive]
pub enum FSGuid {
    SimpleFileSystem = 0x5346_5353_4653_061A_450C_11BF_4EBF_0E06,
    MicrosoftData = 0xC799_26B7_B668_C087_4433_B9E5_EBD0_A0A2,
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

impl TryFrom<u128> for FSGuid {
    type Error = &'static str;

    fn try_from(value: u128) -> Result<Self, Self::Error> {
        match value {
            val if val == Self::SimpleFileSystem as u128 => Ok(Self::SimpleFileSystem),
            val if val == Self::MicrosoftData as u128 => Ok(Self::MicrosoftData),
            _ => Err("Can't find filesystem with that guid"),
        }
    }
}

#[derive(Debug, FromBytes, IntoBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
// should add ending reserved fields
pub struct PartitionEntry {
    // option optimization should make this that any non defined variant is none
    // to test theroy I will edit this freely as a int thru ptr
    pub partion_type_guid: U128,
    pub unique_partion_guid: U128,
    pub starting_lba: U64,
    pub ending_lba: U64,
    pub attributes: U64,
    /// uft 16
    /// TODO: don't hardcode length some implementations have it go beyond 36
    pub partion_name: [U16; 36],
}

impl PartitionEntry {
    pub fn name(&self) -> Option<String> {
        // assumes little endian hardware
        String::from_utf16(&self.partion_name.map(U16::get)).ok()
    }

    /// Returns the get fs of this [`PartitionEntry`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the `partion_type_guid` field
    /// does not contain a known file system
    pub fn get_fs(&self) -> Result<FSGuid, &'static str> {
        FSGuid::try_from(self.partion_type_guid.get())
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
