use core::ascii::Char;

use alloc::sync::Arc;
use diy_os::{device_manager::BlockDevice, multitasking::mutex::Mutex};

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

    pub fn new(device: &Arc<Mutex<dyn BlockDevice>>, partion_lba: u64) -> Self {
        let mut ebr = [0u8; 512];

        device
            .acquire()
            .read_sectors(partion_lba, 1, &mut ebr)
            .unwrap();

        let ebr = unsafe { core::mem::transmute::<[u8; 512], Self>(ebr) };

        // make sure ebr is good before returning fs
        assert!(ebr.valid_signature());

        ebr
    }
}
