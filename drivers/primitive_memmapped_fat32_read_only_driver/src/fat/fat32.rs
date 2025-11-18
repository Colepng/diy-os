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
    pub bpb: super::BIOSParameterBlock,
    pub(super) size_of_fat_in_sectors: u32,
    flags: u16,
    version: FatVersion,
    pub cluster_of_root_dir: super::Cluseter,
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
