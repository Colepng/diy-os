use core::ascii::Char;

#[derive(Debug)]
#[repr(C, packed)]
// rewrite with a sector new type which has a const generics for sector size
pub struct ExtenedBootRecord {
    pub bpb: super::BIOSParameterBlock,
    drive_number: u8,
    _flags_for_windows: u8,
    signature: u8,
    volume_id_serial_number: u32,
    volume_label: [Char; 11],
    sys_id: [Char; 8],
    _boot_code: [u8; 448],
    bootable_signature: u16,
}

impl ExtenedBootRecord {
    pub const fn valid_signature(&self) -> bool {
        self.signature == 0x28 || self.signature == 0x29
    }

    pub fn from_addr(addr: usize) -> &'static Self {
        let ptr: *const Self = core::ptr::with_exposed_provenance(addr);
        // Ptr will always be valid as long as the ramdisk does not move
        // and does not get unmapped
        unsafe { ptr.as_ref().unwrap() }
    }
}
