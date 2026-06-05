#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]
#![feature(strict_provenance_lints)]
#![feature(never_type)]
#![feature(ascii_char)]
#![feature(transmutability)]
#![feature(ascii_char_variants)]
#![warn(
    clippy::pedantic,
    clippy::nursery,
    clippy::perf,
    clippy::style,
    clippy::todo,
    // clippy::undocumented_unsafe_blocks
)]
#![deny(
    clippy::suspicious,
    clippy::correctness,
    clippy::complexity,
    clippy::missing_const_for_fn,
    unsafe_op_in_unsafe_fn,
    fuzzy_provenance_casts
)]
#![allow(
    clippy::return_self_not_must_use,
    clippy::new_without_default,
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    clippy::todo,
    clippy::explicit_deref_methods,
    clippy::missing_panics_doc,
    clippy::fn_to_numeric_cast,
    clippy::unnecessary_box_returns,
    clippy::linkedlist
)]

mod drivers;
mod fat;

use alloc::{boxed::Box, sync::Arc};

use diy_os::{
    device_manager::BlockDevice,
    filesystem::{
        FileSystem,
        gpt::{PartionTableHeader, PartitionEntry},
    },
    multitasking::mutex::Mutex,
    println,
};

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

pub fn fat_setup(device: Arc<Mutex<dyn BlockDevice>>) -> anyhow::Result<Box<dyn FileSystem>> {
    let mut sector_buffer = [0u8; 92];

    let mut drive = device.acquire();

    drive.read_sectors(1, 1, &mut sector_buffer)?;

    // let sector_buffer: [u8; 92] = array::from_fn(|x| sector_buffer[x]);

    let header = unsafe { core::mem::transmute::<[u8; 92], PartionTableHeader>(sector_buffer) };

    println!("header: {header:#?}");
    //
    // // assert!(header.validate(addr));
    //
    // assert!(128 == header.size_of_partion_entry);

    let mut partion_entry = [0u8; 128];

    drive.read_sectors(2, 1, &mut partion_entry)?;

    let partion = unsafe { core::mem::transmute::<[u8; 128], PartitionEntry>(partion_entry) };

    // Has not been called yet
    // unsafe { helper.init(addr.try_into()?, partion.starting_lba.try_into()?) };

    let name = partion.name().unwrap();

    println!("partion {name}, partion {:?}", partion);
    // println!("fs: {:X}", partion.partion_type_guid);
    // let fs = partion.get_fs().unwrap();
    // println!("fs {fs:?}");
    //
    // let partion_addr = helper.addr_from_partion_lba(0);
    let mut bios = [0u8; 36];

    drive.read_sectors(partion.starting_lba, 1, &mut bios)?;

    let bios = unsafe { core::mem::transmute::<[u8; 36], BIOSParameterBlock>(bios) };

    println!("bpb : {bios:?}");

    let fat_type = bios.get_fat_type();

    println!("fat type {:?}", fat_type);

    drop(drive);

    match fat_type {
        FATType::ExFAT => todo!(),
        FATType::FAT12 => todo!(),
        FATType::FAT16 => Ok(drivers::fat16_read_only(partion.starting_lba, device)),
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
