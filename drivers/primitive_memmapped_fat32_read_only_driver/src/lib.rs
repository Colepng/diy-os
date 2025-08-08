#![no_std]
#![feature(strict_provenance_lints)]
#![feature(never_type)]
#![feature(ascii_char)]
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

mod fat;

use diy_os::{
    RAMDISK_INFO,
    filesystem::gpt::{PartionTableHeader, PartitionEntry, mbr::MBR},
    multitasking::exit,
    println,
};

use crate::fat::{BIOSParameterBlock, FATType, fat32::ExtenedBootRecord};

pub fn wrapper() -> ! {
    match primitive_memmapped_fat32_read_only_driver() {
        Err(err) => panic!("{err:?}"),
    }
}

fn primitive_memmapped_fat32_read_only_driver() -> anyhow::Result<!> {
    let (addr, _len) = RAMDISK_INFO.with_mut_ref(|info| {
        let info = info.unwrap();
        (info.addr, info.len)
    });

    let ptr = core::ptr::without_provenance::<MBR>(usize::try_from(addr).unwrap());

    let header_ptr = unsafe { ptr.byte_offset(512) }.cast::<PartionTableHeader>();
    let header = unsafe { header_ptr.read() };

    println!("header: {:#?}", header);
    assert!(header.validate(addr));

    assert!(128 == header.size_of_partion_entry);
    let partion_lba = header.partion_entry_lba;
    let partion_ptr = core::ptr::without_provenance::<PartitionEntry>(usize::try_from(
        addr + (partion_lba * 512),
    )?);

    let slice =
        core::ptr::slice_from_raw_parts(partion_ptr, usize::try_from(header.num_of_partions)?);
    let slice = unsafe { slice.as_ref().unwrap() };

    let partion = slice[0];
    let name = partion.name().unwrap();

    println!("partion {name}, partion {:#?}", partion);

    println!("fs: {:X}", partion.partion_type_guid);
    let fs = partion.get_fs().unwrap();
    println!("fs {fs:?}");

    let boot_recorded_addr = usize::try_from(addr + (512 * partion.starting_lba))?;

    let bios = BIOSParameterBlock::from_addr(boot_recorded_addr);

    let fat_type = bios.get_fat_type();

    println!("fat type {:?}", fat_type);

    assert!(fat_type == FATType::FAT32);

    let fat32_table = ExtenedBootRecord::from_addr(boot_recorded_addr);

    assert!(fat32_table.valid_signature());

    println!("stargin lba {}", partion.starting_lba);

    println!("bios: {:?}", fat32_table);

    // SAFETY: Scheduler is not held
    unsafe { exit() };
}
