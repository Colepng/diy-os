use core::{
    ascii::Char,
    mem::{Assume, TransmuteFrom},
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

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct BIOSParameterBlock {
    _start_code: [u8; 3],
    pub oem_id: [Char; 8],
    pub bytes_per_sec: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sectors: u16,
    pub number_of_fats: u8,
    number_of_roots: u16,
    total_sectors: u16,
    media_descripter: u8,
    pub number_of_sectors_per_fat: u16,
    number_of_sectors_per_track: u16,
    number_of_heads: u16,
    /// the lba at the start of partitio
    pub number_of_hidden_sectors: u32,
    large_sector_count: u32,
}

impl BIOSParameterBlock {
    /// in bytes
    const SIZE_OF_DIR: u16 = 32;

    pub fn from_addr(addr: usize) -> &'static Self {
        let ptr: *const Self = core::ptr::with_exposed_provenance(addr);
        // Ptr will always be valid as long as the ramdisk does not move
        // and does not get unmapped
        unsafe { ptr.as_ref().unwrap() }
    }

    /// Returns the number of sectors the root dir uses
    pub const fn get_size_of_root_dir(&self) -> u16 {
        (self.number_of_roots * Self::SIZE_OF_DIR).div_ceil(self.bytes_per_sec)
    }

    fn get_num_of_data_sectors(&self) -> u32 {
        assert!(self.number_of_fats != 0);

        let total_sectors = if self.total_sectors == 0 {
            self.large_sector_count
        } else {
            self.total_sectors.into()
        };

        let used_sectors: u16 = self.reserved_sectors
            + u16::from(self.number_of_fats) * self.number_of_sectors_per_fat
            + self.get_size_of_root_dir();

        total_sectors - u32::from(used_sectors)
    }

    pub fn first_date_sector(&self) -> u16 {
        self.reserved_sectors
            + (u16::from(self.number_of_fats) * self.number_of_sectors_per_fat)
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
pub struct Cluseter(pub u32);

impl Cluseter {
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
    pub file_name: [u8; 8],
    pub extension: [Char; 3],
    flags: u8,
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

impl Directory {
    const _SIZE_CHECK: () = const {
        assert!(size_of::<Self>() == 32);
    };

    // submit pr for transmutabitly to use const traits
    pub fn cluster(&self) -> Cluseter {
        Cluseter(self.low_2_bytes_of_cluster.into())
    }

    // submit pr for transmutabitly to use const traits
    pub fn cluster_fat32(&self) -> Cluseter {
        unsafe {
            <Cluseter as TransmuteFrom<[u16; 2], { Assume::SAFETY }>>::transmute([
                self.high_2_bytes_of_cluster,
                self.low_2_bytes_of_cluster,
            ])
        }
    }

    pub fn name_as_str(&self) -> String {
        let name = self.file_name.as_ascii().unwrap().as_str().trim();
        let ext = self.extension.as_str();
        alloc::format!("{name}.{ext}")
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct LongFileName {
    letter_offset: u8,
    first_chars: [u16; 5],
    attribute: u8,
    long_entry_type: u8,
    checksum_for_short_name: u8,
    next_chars: [u16; 6],
    zeroed: u16,
    last_charts: [u16; 2],
}

#[derive(Debug)]
#[repr(C, align(4))]
pub struct UnknownEntry {
    unknown: [u8; 11],
    flags: u8,
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

        if self.flags == 0x0F {
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
