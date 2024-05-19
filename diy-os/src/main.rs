#![no_std]
#![no_main]
#![feature(optimize_attribute)]
#![feature(custom_test_frameworks)]
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
use diy_os::{
    allocator, hlt_loop, init,
    memory::{self, BootInfoFrameAllocator},
    println, timer,
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

    println!("Hello, world!");

    println!("going to sleep");

    timer::sleep(1000);

    println!("wakign up");

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
