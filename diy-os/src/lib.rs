#![no_std]
#![crate_type = "rlib"]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_harness_main"]
#![feature(abi_x86_interrupt)]
#![feature(const_mut_refs)]
#![feature(negative_impls)]
#![feature(ascii_char)]
#![feature(ascii_char_variants)]
#![feature(anonymous_lifetime_in_impl_trait)]
#![feature(const_refs_to_static)]
#![feature(strict_provenance)]
#![feature(exposed_provenance)]
#![feature(layout_for_ptr)]
#![feature(naked_functions)]
#![feature(str_from_raw_parts)]
#![warn(
    clippy::pedantic,
    clippy::nursery,
    clippy::perf,
    clippy::style,
    clippy::todo,
    clippy::undocumented_unsafe_blocks
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
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    clippy::explicit_deref_methods
)]

use bootloader_api::BootInfo;
use core::panic::PanicInfo;
use memory::BootInfoFrameAllocator;
use timer::SystemTimerError;
use x86_64::structures::paging::{mapper::MapToError, OffsetPageTable, Size4KiB};

#[cfg(test)]
use bootloader_api::entry_point;
extern crate alloc;

pub mod allocator;
pub mod console;
pub mod elf;
pub mod errors;
pub mod filesystem;
pub mod framebuffer;
pub mod gdt;
pub mod interrupts;
pub mod memory;
pub mod pit;
pub mod ps2;
pub mod serial;
pub mod spinlock;
pub mod syscalls;
pub mod timer;
pub mod usermode;

pub trait Testable {
    #[allow(clippy::unused_unit)]
    fn run(&self) -> ();
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
}

pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    exit_qemu(QemuExitCode::Success);
}

pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failed);
    hlt_loop();
}

#[cfg(test)]
entry_point!(test_main);

/// Entry point for `cargo test`
#[cfg(test)]
#[no_mangle]
fn test_main(boot_info: &'static mut BootInfo) -> ! {
    init(boot_info, 10);
    test_harness_main();
    hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}

#[derive(thiserror::Error, Debug)]
pub enum InitError {
    #[error("Failed to setup heap because {0:?}")]
    FailedToSetupHeap(MapToError<Size4KiB>),
    #[error("Failed to setup system timer")]
    FailedToSetupSystemTimer(#[from] SystemTimerError),
}

/// # Errors
/// Will return [`InitError::FailedToSetupHeap`] if it could not map the pages for the heap.
/// Will also return [`InitError::FailedToSetupSystemTimer`] if the system timer was already owned
/// # Panics
/// Will panic if no physical memory offset could be found
pub fn init(
    boot_info: &'static mut BootInfo,
    frequency: u32,
) -> Result<
    (
        &'static BootInfo,
        BootInfoFrameAllocator,
        OffsetPageTable<'static>,
    ),
    InitError,
> {
    println!("Entering init");

    // Setup Allocator first for error propagation with anyhow
    let offset_addr =
        x86_64::VirtAddr::new(boot_info.physical_memory_offset.into_option().unwrap());

    // setup the heap
    let mut frame_allocator =
        unsafe { memory::BootInfoFrameAllocator::init(&boot_info.memory_regions) };
    let mut mapper = unsafe { memory::init(offset_addr) };
    allocator::setup_heap(&mut mapper, &mut frame_allocator)
        .map_err(InitError::FailedToSetupHeap)?;

    if let Some(framebuffer) = boot_info.framebuffer.take() {
        framebuffer::init(framebuffer);
        println!("Framebuffer Initialized");
    }

    gdt::init();
    println!("GDT Initialized");

    interrupts::init_idt();
    println!("IDT Initialized");
    unsafe { interrupts::PICS.acquire().initialize() };
    println!("PICS Initialized");
    interrupts::unmask();
    println!("Interrupts Unmasked");
    x86_64::instructions::interrupts::enable();
    println!("Interrupts Enabled");
    timer::setup_system_timer(frequency)?;
    println!("System timer Initialized");

    Ok((boot_info, frame_allocator, mapper))
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
