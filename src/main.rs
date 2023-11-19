#![no_std]
#![no_main]
#![feature(optimize_attribute)]
#![feature(custom_test_frameworks)]
#![test_runner(diy_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use diy_os::{
    hlt_loop, init,
    memory::{self, BootInfoFrameAllocator},
    println,
};
use x86_64::structures::paging::{Page, PageTable, Translate};

entry_point!(main);

#[no_mangle]
fn main(boot_info: &'static BootInfo) -> ! {
    use x86_64::VirtAddr;

    println!("hello world");
    init();

    let physicals_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);

    let mut mapper = unsafe { diy_os::memory::init(physicals_mem_offset) };
    let mut allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    let page = Page::containing_address(VirtAddr::new(0));
    memory::create_example_mapping(page, &mut mapper, &mut allocator);

    let page_ptr: *mut u64 = page.start_address().as_mut_ptr();
    unsafe { page_ptr.offset(400).write_volatile(0x_f021_f077_f065_f04e) };

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
