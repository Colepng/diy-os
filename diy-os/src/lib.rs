#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]
#![feature(abi_x86_interrupt)]
#![feature(negative_impls)]
#![feature(ascii_char)]
#![feature(ascii_char_variants)]
#![feature(anonymous_lifetime_in_impl_trait)]
#![feature(layout_for_ptr)]
#![feature(str_from_raw_parts)]
#![feature(transmutability)]
#![feature(strict_provenance_lints)]
#![feature(slice_ptr_get)]
#![feature(ptr_metadata)]
#![feature(variant_count)]
#![feature(iter_collect_into)]
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
)]

use bootloader_api::BootInfo;
use memory::BootInfoFrameAllocator;
use timer::SystemTimerError;
use x86_64::structures::paging::{OffsetPageTable, Size4KiB, mapper::MapToError};

extern crate alloc;

#[cfg(not(test))]
pub mod allocator;
pub mod collections;
pub mod console;
pub mod elf;
pub mod errors;
pub mod filesystem;
pub mod framebuffer;
pub mod gdt;
pub mod human_input_devices;
pub mod interrupts;
pub mod memory;
pub mod multitasking;
pub mod pit;
pub mod ps2;
pub mod serial;
pub mod spinlock;
pub mod syscalls;
pub mod timer;
pub mod usermode;
pub mod volatile;
pub mod log;

#[derive(thiserror::Error, Debug)]
pub enum InitError {
    #[error("Failed to setup heap because {0:?}")]
    FailedToSetupHeap(MapToError<Size4KiB>),
    #[error("Failed to setup system timer")]
    FailedToSetupSystemTimer(#[from] SystemTimerError),
}

/// frequency is in hz
/// # Errors
/// Will return [`InitError::FailedToSetupHeap`] if it could not map the pages for the heap.
/// Will also return [`InitError::FailedToSetupSystemTimer`] if the system timer was already owned
/// # Panics
/// Will panic if no physical memory offset could be found
pub fn kernel_early(
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
    // Setup Allocator first for error propagation with anyhow
    let offset_addr =
        x86_64::VirtAddr::new(boot_info.physical_memory_offset.into_option().unwrap());

    // setup the heap
    let mut frame_allocator =
        unsafe { memory::BootInfoFrameAllocator::init(&boot_info.memory_regions) };
    let mut mapper = unsafe { memory::init(offset_addr) };
    #[cfg(not(test))]
    allocator::setup_heap(&mut mapper, &mut frame_allocator)
        .map_err(InitError::FailedToSetupHeap)?;

    log::info("Heap setup, can start logging");

    if let Some(framebuffer) = boot_info.framebuffer.take() {
        framebuffer::init(framebuffer);
        log::info("Initialized framebuffer");
    }

    gdt::init();
    log::info("The GDT was initialized");

    interrupts::init_idt();
    log::info("The IDT was initialized");

    unsafe { interrupts::PICS.acquire().initialize() };
    log::info("The PIC was initialized");
    interrupts::unmask();
    log::info("Unmasked interrupts");

    x86_64::instructions::interrupts::enable();
    log::info("Interrupts Enabled");

    timer::setup_system_timer(frequency)?;
    log::info("System timer Initialized");

    Ok((boot_info, frame_allocator, mapper))
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[test]
fn test_testing() {
    assert!(true);
}
