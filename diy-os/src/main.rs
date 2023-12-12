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

use bootloader_api::{entry_point, BootInfo};
use core::panic::PanicInfo;
use diy_os::{allocator, hlt_loop, init, memory::BootInfoFrameAllocator, println};

entry_point!(main);

#[no_mangle]
extern "Rust" fn main(boot_info: &'static mut BootInfo) -> ! {
    use x86_64::VirtAddr;
    
    // println!("hello world");
    // init();
    //
    // let physicals_mem_offset =
    //     VirtAddr::new(boot_info.physical_memory_offset.into_option().unwrap());
    // let mut mapper = unsafe { diy_os::memory::init(physicals_mem_offset) };
    // let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_regions) };
    //
    // allocator::setup_heap(&mut mapper, &mut frame_allocator).expect("failed to setup heap");
    //
    // #[cfg(test)]
    // test_main();
    //
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
