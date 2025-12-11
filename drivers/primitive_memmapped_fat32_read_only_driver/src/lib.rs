#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]
#![feature(strict_provenance_lints)]
#![feature(never_type)]
#![feature(ascii_char)]
#![feature(transmutability)]
#![feature(maybe_uninit_uninit_array_transpose)]
#![feature(maybe_uninit_as_bytes)]
#![feature(ptr_as_uninit)]
#![feature(ub_checks)]
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

use core::{cell::OnceCell, fmt};

use diy_os::{
    RAMDISK_INFO,
    filesystem::gpt::{PartionTableHeader, PartitionEntry, mbr::MBR},
    multitasking::{exit, mutex::Mutex},
    println,
};

use crate::fat::{BIOSParameterBlock, Cluseter, FATType};

extern crate alloc;

#[cfg(target_os = "none")]
pub fn wrapper() {
    match fat_setup() {
        Err(err) => panic!("{err:?}"),
        Ok(_) => unsafe {},
    }
}

#[cfg(not(target_os = "none"))]
pub fn wrapper() {
    match fat_setup() {
        Err(err) => panic!("{err:?}"),
        Ok(_) => {}
    }
}

struct Helper {
    base_addr: OnceCell<usize>,
    partion_lba: OnceCell<usize>,
}

impl Helper {
    pub const fn new() -> Self {
        Self {
            base_addr: OnceCell::new(),
            partion_lba: OnceCell::new(),
        }
    }

    pub unsafe fn init(&self, base_addr: usize, partion_lba: usize) {
        self.base_addr.set(base_addr).unwrap();
        self.partion_lba.set(partion_lba).unwrap();
    }

    pub fn addr_from_partion_lba(&self, lba: usize) -> usize {
        ((lba + self.partion_lba.get().unwrap()) * 512) + self.base_addr.get().unwrap()
    }

    /// crates some type at the start of a LBA, from the start of the drive
    pub fn ptr_from_lba<T, N>(&self, lba: N) -> *const T
    where
        N: TryInto<usize>,
        <N as TryInto<usize>>::Error: fmt::Debug,
    {
        let addr = lba.try_into().unwrap() + self.base_addr.get().unwrap();
        core::ptr::with_exposed_provenance(addr)
    }

    /// crates some type at the start of a LBA, from the start of the drive
    pub fn ptr_from_partion_lba<T, N>(&self, lba: N) -> *const T
    where
        N: TryInto<usize>,
        <N as TryInto<usize>>::Error: fmt::Debug,
    {
        let addr = ((lba.try_into().unwrap() + self.partion_lba.get().unwrap()) * 512)
            + self.base_addr.get().unwrap();
        core::ptr::with_exposed_provenance(addr)
    }
}

fn fat_setup() -> anyhow::Result<()> {
    let (addr, _len) = RAMDISK_INFO.with_mut_ref(|info| {
        let info = info.unwrap();
        (info.addr, info.len)
    });

    let helper = Helper::new();

    let ptr = core::ptr::with_exposed_provenance::<MBR>(usize::try_from(addr).unwrap());

    let header_ptr = unsafe { ptr.byte_offset(512) }.cast::<PartionTableHeader>();
    let header = unsafe { header_ptr.read() };

    println!("header: {:?}", header);
    println!("addr: {addr}");
    assert!(header.validate(addr));

    assert!(128 == header.size_of_partion_entry);
    let partion_entry_lba = header.partion_entry_lba;

    let partion_addr = addr + partion_entry_lba * 512;
    let partion_ptr: *const PartitionEntry =
        core::ptr::with_exposed_provenance(partion_addr.try_into().unwrap());

    let partion = unsafe { partion_ptr.read_unaligned() };

    // Has not been called yet
    unsafe { helper.init(addr.try_into()?, partion.starting_lba.try_into()?) };

    let name = partion.name().unwrap();

    println!("partion {name}, partion {:?}", partion);

    println!("fs: {:X}", partion.partion_type_guid);
    let fs = partion.get_fs().unwrap();
    println!("fs {fs:?}");

    let partion_addr = helper.addr_from_partion_lba(0);

    let bios = BIOSParameterBlock::from_addr(partion_addr);

    println!("bpb : {bios:?}");

    let fat_type = bios.get_fat_type();

    println!("fat type {:?}", fat_type);

    match fat_type {
        FATType::ExFAT => todo!(),
        FATType::FAT12 => todo!(),
        FATType::FAT16 => {
            drivers::primitive_memmapped_fat16_read_only_driver(partion_addr, &helper);
        }
        FATType::FAT32 => {
            todo!()
            // drivers::primitive_memmapped_fat32_read_only_driver(partion_addr, boot_recorded_addr)
        }
    }

    Ok(())
}

pub fn get_table_value(cluster: Cluseter, bios: &BIOSParameterBlock, partion_addr: u64) -> u32 {
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
