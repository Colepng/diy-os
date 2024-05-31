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
use core::{arch::{asm, global_asm}, panic::PanicInfo};
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

    // println!("going to sleep");
    //
    // timer::sleep(1000);
    //
    // println!("wakign up");
    //
    print("print sys call");

    hlt_loop();
}


/// Parameters to functions are passed in the registers rdi, rsi, rdx, rcx, r8, r9, and further 
/// values are passed on the stack in reverse order
///
/// rax return
///
unsafe extern "sysv64" fn sys_call<T, F>(call: u64, t: usize, f: usize) {//where T: , F: core::fmt::Display {
    unsafe {
        asm!("mov rsi, rsi");
        asm!("mov rdx, rdx");
        asm!("mov rax, {0}", in(reg) call);
    }

    unsafe { x86_64::instructions::interrupts::software_interrupt::<0x80>() };
}

fn print(str: &str) {
    let len = str.len();
    let ptr = str.as_ptr();

    unsafe { 
        sys_call::<u64, u64>(0, ptr as usize, len)
    }
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
