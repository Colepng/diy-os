#![no_std]
#![no_main]
#![feature(optimize_attribute)]
#![feature(custom_test_frameworks)]
#![feature(naked_functions)]
#![feature(asm_const)]
#![feature(pointer_is_aligned_to)]
#![test_runner(diy_os::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![warn(clippy::pedantic, clippy::nursery, clippy::perf, clippy::style)]
#![deny(
    clippy::suspicious,
    clippy::correctness,
    clippy::complexity,
    clippy::missing_const_for_fn,
    unsafe_op_in_unsafe_fn
)]

extern crate alloc;

use bootloader_api::{
    config::{Mapping, Mappings},
    entry_point, BootInfo, BootloaderConfig,
};
use core::panic::PanicInfo;
use core::{ptr, slice};
use diy_os::{
    allocator::{self, fixed_size_block},
    elf,
    filesystem::{self, ustar},
    hlt_loop, init,
    memory::{self, BootInfoFrameAllocator},
    println, multitasking::{self, usermode, Process},
};
use x86_64::{
    structures::paging::{FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB},
    VirtAddr,
};

static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    let mut mappings = Mappings::new_default();
    mappings.physical_memory = Some(Mapping::Dynamic);
    config.mappings = mappings;

    config
};

entry_point!(main, config = &BOOTLOADER_CONFIG);

#[no_mangle]
extern "Rust" fn main(mut boot_info: &'static mut BootInfo) -> ! {
    boot_info = init(boot_info);

    let offset_addr =
        x86_64::VirtAddr::new(boot_info.physical_memory_offset.into_option().unwrap());

    // setup the heap
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_regions) };
    let mut mapper = unsafe { memory::init(offset_addr) };
    allocator::setup_heap(&mut mapper, &mut frame_allocator).expect("Failed to setup heap fuck u");

    let ramdisk_addr = boot_info.ramdisk_addr.into_option().unwrap();
    let ramdisk = unsafe { ustar::Ustar::new(ramdisk_addr) };

    println!("Hello, world!");

    let elf_file = &ramdisk.get_files()[0];

    // let exit_code = usermode::load_elf_and_jump_into_it(elf_file, &mut mapper, &mut frame_allocator).unwrap();
    // println!("{:?}", exit_code);
    println!("loading");
    let temp = usermode::load_elf(elf_file, &mut mapper, &mut frame_allocator).unwrap();
    println!("{temp:?}");

    let mut sched = multitasking::SCHEDULER.acquire();
    let process = Process::new(temp.0, temp.1);
    sched.spawn_new(process);
    core::mem::drop(sched);
    println!("spawned");

    multitasking::switch_to_task();

    hlt_loop();
}

/// This function is called on panic.
#[cfg(not(test))] // new attribute
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    diy_os::test_panic_handler(info)
}

// test to make sure tests won't panic
#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}
