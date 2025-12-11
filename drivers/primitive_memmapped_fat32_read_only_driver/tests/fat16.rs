#![feature(vec_into_raw_parts)]

use std::fs;

use diy_os::{RAMDISK_INFO, RamdiskInfo};
use primitive_memmapped_fat32_read_only_driver::wrapper;

fn setup_ramdisk() -> (*mut u8, usize, usize) {
    let ramdisk_path = "/home/cole/Documents/Projects/diy-os/drivers/primitive_memmapped_fat32_read_only_driver/tests/ramdisk.img";
    // let ramdisk_path = "/home/cole/Documents/Projects/diy-os/sfs.img";

    let ramdisk = fs::read(ramdisk_path).unwrap();

    let raw_parts = ramdisk.into_raw_parts();

    let addr = raw_parts.0.expose_provenance();

    println!("addr: {addr}");

    let info = RamdiskInfo {
        addr: addr.try_into().unwrap(),
        len: raw_parts.1.try_into().unwrap(),
    };

    RAMDISK_INFO.with_mut_ref(|ramdisk_info| {
        *ramdisk_info = Some(info);
    });

    raw_parts
}

fn cleanup_ramdisk(raw_parts: (*mut u8, usize, usize)) {
    unsafe { Vec::from_raw_parts(raw_parts.0, raw_parts.1, raw_parts.2) };
}

#[test]
fn main() {
    let raw_parts = setup_ramdisk();

    wrapper();

    cleanup_ramdisk(raw_parts);
}
