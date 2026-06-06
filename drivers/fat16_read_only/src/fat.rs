use bitflags::bitflags;
use core::{
    ascii::Char,
    mem::{Assume, TransmuteFrom},
};
use zerocopy::{
    FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned,
    little_endian::{U16, U32},
};

use alloc::string::String;
use either::Either::{self, Left, Right};

use crate::fat::fat32::ExtenedBootRecord;

pub mod fat16;
pub mod fat32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FATType {
    ExFAT,
    FAT12,
    FAT16,
    FAT32,
}

#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, Unaligned, KnownLayout)]
#[repr(C)]
pub struct BIOSParameterBlock {
    _start_code: [u8; 3],
    pub oem_id: [u8; 8],
    pub bytes_per_sec: U16,
    pub sectors_per_cluster: u8,
    pub reserved_sectors: U16,
    pub number_of_fats: u8,
    pub number_of_roots: U16,
    pub total_sectors: U16,
    pub media_descripter: u8,
    pub number_of_sectors_per_fat: U16,
    pub number_of_sectors_per_track: U16,
    pub number_of_heads: U16,
    /// the lba at the start of partition
    pub number_of_hidden_sectors: U32,
    pub large_sector_count: U32,
}

impl BIOSParameterBlock {
    /// in bytes
    const SIZE_OF_DIR: u16 = 32;

    /// Returns the number of sectors the root dir uses
    pub const fn get_size_of_root_dir(&self) -> u16 {
        (self.number_of_roots.get() * Self::SIZE_OF_DIR).div_ceil(self.bytes_per_sec.get())
    }

    fn get_num_of_data_sectors(&self) -> u32 {
        assert!(self.number_of_fats != 0);

        let total_sectors = if self.total_sectors == 0 {
            self.large_sector_count
        } else {
            self.total_sectors.into()
        };

        let used_sectors: u16 = self.reserved_sectors.get()
            + u16::from(self.number_of_fats) * self.number_of_sectors_per_fat.get()
            + self.get_size_of_root_dir();

        total_sectors.get() - u32::from(used_sectors)
    }

    pub fn first_date_sector(&self) -> u16 {
        self.reserved_sectors.get()
            + (u16::from(self.number_of_fats) * self.number_of_sectors_per_fat.get())
            + self.get_size_of_root_dir()
    }

    fn first_date_sector_fat32(&self, ebr: &ExtenedBootRecord) -> u32 {
        u32::from(self.reserved_sectors)
            + (u32::from(self.number_of_fats) * ebr.size_of_fat_in_sectors)
            + u32::from(self.get_size_of_root_dir())
    }

    fn get_num_of_clusters(&self) -> u32 {
        self.get_num_of_data_sectors() / u32::from(self.sectors_per_cluster)
    }

    pub fn get_fat_type(&self) -> FATType {
        if self.bytes_per_sec == 0 {
            FATType::ExFAT
        } else if self.get_num_of_clusters() < 4085 {
            FATType::FAT12
        } else if self.get_num_of_clusters() < 65525 {
            FATType::FAT16
        } else {
            FATType::FAT32
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Cluster(pub u32);

impl Cluster {
    pub fn first_sector_of_cluster_fat32(
        self,
        boot: &BIOSParameterBlock,
        ebr: &ExtenedBootRecord,
    ) -> u32 {
        (self.0 - 2) * u32::from(boot.sectors_per_cluster) + boot.first_date_sector_fat32(ebr)
    }

    pub fn first_sector_of_cluster(self, boot: &BIOSParameterBlock) -> u32 {
        (self.0 - 2) * u32::from(boot.sectors_per_cluster) + u32::from(boot.first_date_sector())
    }
}

#[derive(Clone, Copy)]
pub struct Time(u16);

impl Time {
    pub const fn hour(self) -> u16 {
        self.0 >> 11
    }

    pub const fn minute(self) -> u16 {
        (self.0 >> 6) & 0b11_1111
    }

    pub const fn second(self) -> u16 {
        (self.0 & 0b1_1111) * 2
    }
}

impl core::fmt::Debug for Time {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Time")
            .field("hour", &self.hour())
            .field("minute", &self.minute())
            .field("seconds", &self.second())
            .finish()
    }
}

#[repr(u16)]
#[derive(Debug)]
#[allow(dead_code)]
pub enum Month {
    January = 1,
    February = 2,
    March = 3,
    April = 4,
    May = 5,
    June = 6,
    July = 7,
    August = 8,
    September = 9,
    October = 10,
    November = 11,
    December = 12,
}

impl Month {
    // The caller must insure that the value is between 1 and 12 inclusive
    unsafe fn new_unchecked(value: u16) -> Self {
        assert!((1..=12).contains(&value));

        // # Safety value must fall within range of enum, checked by the assert
        // above
        unsafe { core::mem::transmute::<u16, Self>(value) }
    }
}

#[derive(Clone, Copy)]
pub struct Date(u16);

impl Date {
    const fn year(self) -> u16 {
        (self.0 >> 9) + 1980
    }

    fn month(self) -> Month {
        // Hope file is just right, this is unsafe
        unsafe { Month::new_unchecked((self.0 >> 5) & 0b1111) }
    }

    const fn day(self) -> u16 {
        self.0 & 0b1_1111
    }
}

impl core::fmt::Debug for Date {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Date")
            .field("year", &self.year())
            .field("month", &self.month())
            .field("day", &self.day())
            .finish()
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Directory {
    pub file_name: [Char; 8],
    pub extension: [Char; 3],
    pub flags: EntryFlags,
    _reserved: u8,
    creation_time_hund_seconds: u8,
    creation_time: Time,
    creation_date: Date,
    last_opened: Date,
    high_2_bytes_of_cluster: u16,
    last_modified_time: Time,
    last_modified_date: Date,
    low_2_bytes_of_cluster: u16,
    pub size_in_bytes: u32,
}

bitflags! {
    // Attributes can be applied to flags types
    // #[repr(transparent)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct EntryFlags: u8 {
        const ReadOnly = 0x01;
        const Hidden = 0x02;
        const System = 0x04;
        const VolumeId = 0x08;
        const Directory = 0x10;
        const Archive = 0x20;
    }
}

impl Directory {
    const _SIZE_CHECK: () = const {
        assert!(size_of::<Self>() == 32);
    };

    // submit pr for transmutabitly to use const traits
    pub fn cluster(&self) -> Cluster {
        Cluster(self.low_2_bytes_of_cluster.into())
    }

    // submit pr for transmutabitly to use const traits
    #[allow(dead_code)]
    pub fn cluster_fat32(&self) -> Cluster {
        unsafe {
            <Cluster as TransmuteFrom<[u16; 2], { Assume::SAFETY }>>::transmute([
                self.high_2_bytes_of_cluster,
                self.low_2_bytes_of_cluster,
            ])
        }
    }

    #[allow(dead_code)]
    pub fn name_as_str(&self) -> String {
        let name = self.file_name.as_str().trim();
        let ext = self.extension.as_str();
        alloc::format!("{name}.{ext}")
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct LongFileName {
    letter_offset: u8,
    first_chars: [u16; 5],
    flags: EntryFlags,
    long_entry_type: u8,
    checksum_for_short_name: u8,
    next_chars: [u16; 6],
    zeroed: u16,
    last_chars: [u16; 2],
}

// Replace with proper handling of 16bit wide chars
impl LongFileName {
    pub fn name_as_str(&self) -> String {
        let first_chars = self.first_chars.map(|char| {
            if char == 0xFF || char == 0 {
                Char::Space
            } else {
                Char::from_u8(u8::try_from(char).unwrap_or(u8::MAX)).unwrap_or(Char::Space)
            }
        });

        let first_chars = first_chars.as_str().trim();

        let next_chars = self.next_chars.map(|char| {
            if char == 0xFF || char == 0 {
                Char::Space
            } else {
                Char::from_u8(u8::try_from(char).unwrap_or(u8::MAX)).unwrap_or(Char::Space)
            }
        });

        let next_chars = next_chars.as_str().trim();

        let last_chars = self.last_chars.map(|char| {
            if char == 0xFF || char == 0 {
                Char::Space
            } else {
                Char::from_u8(u8::try_from(char).unwrap_or(u8::MAX)).unwrap_or(Char::Space)
            }
        });

        let last_chars = last_chars.as_str().trim();
        alloc::format!("{first_chars}{next_chars}{last_chars}")
    }
}

#[derive(Debug)]
#[repr(C, align(4))]
pub struct UnknownEntry {
    unknown: [u8; 11],
    flags: EntryFlags,
    _unknown2: [u8; 16],
    _unused: [u8; 4],
}

impl UnknownEntry {
    const _SIZE_CHECK: () = const {
        assert!(size_of::<Self>() == 32);
    };

    pub const fn empty(&self) -> bool {
        self.unknown[0] == 0
    }

    pub const fn unused(&self) -> bool {
        self.unknown[0] == 0xE5
    }

    pub fn get_entry(&self) -> Option<Either<Directory, LongFileName>> {
        assert!(self.unknown[0] != 0);

        if self.unknown[0] == 0xE5 {
            return None;
        }

        if self.flags
            == EntryFlags::ReadOnly | EntryFlags::Hidden | EntryFlags::System | EntryFlags::VolumeId
        {
            Some(Right(*self.as_long_name_unchecked()))
        } else {
            Some(Left(*self.as_directory_unchecked()))
        }
    }

    pub const fn as_directory_unchecked(&self) -> &Directory {
        unsafe {
            core::ptr::from_ref(self)
                .cast::<Directory>()
                .as_ref()
                .unwrap()
        }
    }

    pub const fn as_long_name_unchecked(&self) -> &LongFileName {
        unsafe {
            core::ptr::from_ref(self)
                .cast::<LongFileName>()
                .as_ref()
                .unwrap()
        }
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct Sector(pub [UnknownEntry; 16]);

impl Sector {
    const _SIZE_CHECK: () = const {
        assert!(size_of::<Self>() == 512);
    };
}
