#![no_std]
#![no_main]
#![feature(optimize_attribute)]
#![feature(custom_test_frameworks)]
#![test_runner(diy_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use core::panic::PanicInfo;
use alloc::{boxed::Box, rc::Rc, vec};
use bootloader::{BootInfo, entry_point};
use diy_os::{init, println, hlt_loop, memory::{self, BootInfoFrameAllocator}, allocator};

entry_point!(main);

#[no_mangle]
fn main(boot_info: &'static BootInfo) -> ! {
    use x86_64::VirtAddr;

    println!("hello world");
    init();

    let physicals_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { diy_os::memory::init(physicals_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    allocator::setup_heap(&mut mapper, &mut frame_allocator).expect("failed to setup heap");

    // create a reference counted vector -> will be freed when count reaches 0
    let reference_counted = Rc::new(vec![1, 2, 3]);
    let cloned_reference = reference_counted.clone();
    println!("current reference count is {}", Rc::strong_count(&cloned_reference));
    core::mem::drop(reference_counted);
    println!("reference count is {} now", Rc::strong_count(&cloned_reference));

    // // create a dynamically sized vector
    // let mut vec = Vec::new();
    // for i in 0..500 {
    //     vec.push(i);
    // }
    // println!("vec at {:?}", vec.as_slice());

    #[cfg(test)]
    test_main();

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
