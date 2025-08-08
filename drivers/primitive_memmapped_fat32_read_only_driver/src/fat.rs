use core::ascii::Char;

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
    secters_per_cluster: u8,
    reserved_sectors: u16,
    number_of_fats: u8,
    number_of_roots: u16,
    total_sectors: u16,
    media_descripter: u8,
    number_of_sectors_per_fat: u16,
    number_of_sectors_per_track: u16,
    number_of_heads: u16,
    /// the lba at the start of partitio
    pub number_of_hidden_sectors: u32,
    large_sector_count: u32,
}

impl BIOSParameterBlock {
    /// in bytes
    const SIZE_OF_DIR: u16 = 32;

    pub const fn from_addr(addr: usize) -> &'static Self {
        let ptr: *const Self = core::ptr::without_provenance(addr);
        // Ptr will always be valid as long as the ramdisk does not move
        // and does not get unmapped
        unsafe { ptr.as_ref().unwrap() }
    }

    /// Returns the number of sectors the root dir uses
    const fn get_size_of_root_dir(&self) -> u16 {
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

    fn get_num_of_clusters(&self) -> u32 {
        self.get_num_of_data_sectors() / u32::from(self.secters_per_cluster)
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

pub mod fat32 {
    use core::ascii::Char;

    #[derive(Debug, Clone, Copy)]
    #[repr(C)]
    pub struct FatVersion {
        major_version: u8,
        minor_version: u8,
    }

    #[derive(Debug)]
    #[repr(C, packed)]
    // rewrite with a sector new type which has a const generics for sector size
    pub struct ExtenedBootRecord {
        bpb: super::BIOSParameterBlock,
        size_of_fat_in_sectors: u32,
        flags: u16,
        version: FatVersion,
        cluster_of_root_dir: u32,
        sector_of_fs_info: u16,
        sector_of_backup_boot: u16,
        _reserved: [u8; 12],
        drive_number: u8,
        _flags_for_windows: u8,
        signature: u8,
        volume_id_serial_number: u32,
        volume_label: [Char; 11],
        sys_id: [Char; 8],
        _boot_code: [u8; 420],
        bootable_signature: u16,
    }

    impl ExtenedBootRecord {
        pub const fn valid_signature(&self) -> bool {
            self.signature == 0x28 || self.signature == 0x29
        }

        pub const fn from_addr(addr: usize) -> &'static Self {
            let ptr: *const Self = core::ptr::without_provenance(addr);
            // Ptr will always be valid as long as the ramdisk does not move
            // and does not get unmapped
            unsafe { ptr.as_ref().unwrap() }
        }
    }
}
