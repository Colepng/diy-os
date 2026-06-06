#![no_std]
#![no_main]
#![feature(strict_provenance_lints)]
#![feature(never_type)]
#![feature(ascii_char)]
#![feature(transmutability)]
#![feature(ascii_char_variants)]

mod drivers;
mod fat;

use alloc::{boxed::Box, sync::Arc};

use diy_os::{
    device_manager::{BlockDevice, BlockDeviceError},
    filesystem::{FileSystem, gpt::PartitionEntry},
    multitasking::mutex::Mutex,
    println,
};
use zerocopy::{FromZeros, IntoBytes};

use crate::fat::{BIOSParameterBlock, Cluster, FATType};

extern crate alloc;
//
// #[cfg(target_os = "none")]
// pub fn wrapper() {
//     match fat_setup() {
//         Err(err) => panic!("{err:?}"),
//         Ok(_) => unsafe {},
//     }
// }
//
// #[cfg(not(target_os = "none"))]
// pub fn wrapper() {
//     match fat_setup() {
//         Err(err) => panic!("{err:?}"),
//         Ok(_) => {}
//     }
// }

/// Reads the BIOS Parameter Block from a FAT partition, detects the FAT
/// variant, and instantiates the appropriate filesystem driver.
///
/// `partion` must reference a partition whose filesystem GUID is
/// [`FSGuid::MicrosoftData`]. The BPB is read from the partition's starting LBA.
///
/// # Errors
///
/// Returns [`BlockDeviceError`] if reading the BPB sector fails.
pub fn fat_setup(
    device: Arc<Mutex<dyn BlockDevice>>,
    partion: &PartitionEntry,
) -> Result<Box<dyn FileSystem>, BlockDeviceError> {
    let mut drive = device.acquire();

    let mut bios = BIOSParameterBlock::new_zeroed();

    drive.read_sectors(partion.starting_lba.get(), 1, bios.as_mut_bytes())?;

    println!("bpb : {bios:?}");

    let fat_type = bios.get_fat_type();

    println!("fat type {:?}", fat_type);

    drop(drive);

    match fat_type {
        FATType::ExFAT => todo!(),
        FATType::FAT12 => todo!(),
        FATType::FAT16 => Ok(drivers::fat16_read_only(partion.starting_lba.get(), device)),
        FATType::FAT32 => {
            todo!()
            // drivers::primitive_memmapped_fat32_read_only_driver(partion_addr, boot_recorded_addr)
        }
    }
}

pub fn get_table_value(cluster: Cluster, bios: &BIOSParameterBlock, partion_addr: u64) -> u32 {
    let mut fat_table: [u32; 128] = [0; 128];
    let offset = cluster.0 * 4;
    let fat_sector = u32::from(bios.reserved_sectors) + (offset / 512);

    let addr = partion_addr + u64::from(fat_sector * 512);

    let ptr: *const [u32; 128] = core::ptr::without_provenance(usize::try_from(addr).unwrap());

    unsafe { core::ptr::copy_nonoverlapping(ptr, core::ptr::from_mut(&mut fat_table), 1) };

    let ent_offset = offset % 512;

    let table_value: u32 = fat_table[usize::try_from(ent_offset).unwrap()];

    println!("{:#?}", fat_table);

    // clear the 4 high bits
    table_value & 0x0FFF_FFFF
}

// fn get_entire_slice_from_cluster(
//     cluster: Cluseter,
//     bios: &BIOSParameterBlock,
//     partion_addr: u64,
// ) -> &'static [Sector] {
//     let sector = cluster.first_sector_of_cluster(bios);
//     let num_of_sectors = bios.sectors_per_cluster;
//
//     let addr = partion_addr + u64::from(sector * 512);
//
//     // [u32; 128] is equal to a sector size, 512 bytes
//     let ptr: *const Sector = core::ptr::without_provenance(usize::try_from(addr).unwrap());
//     let slice = core::ptr::slice_from_raw_parts(ptr, num_of_sectors.into());
//
//     unsafe { slice.as_ref().unwrap() }
// }
