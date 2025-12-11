#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]
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
#![feature(never_type)]
#![feature(sync_unsafe_cell)]
#![feature(box_as_ptr)]
#![feature(const_trait_impl)]
#![feature(const_convert)]
#![feature(iter_array_chunks)]
#![feature(int_from_ascii)]
#![feature(test)]
#![feature(pointer_is_aligned_to)]
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

use bootloader_api::BootInfo;
use log::info;
use memory::BootInfoFrameAllocator;
use timer::SystemTimerError;
use x86_64::structures::paging::{OffsetPageTable, Size4KiB, mapper::MapToError};

use crate::multitasking::mutex::Mutex;

extern crate alloc;

#[cfg(target_os = "none")]
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
pub mod logger;
pub mod memory;
pub mod multitasking;
pub mod pit;
pub mod ps2;
pub mod serial;
pub mod syscalls;
pub mod timer;
pub mod usermode;

#[derive(Debug, Clone, Copy)]
pub struct RamdiskInfo {
    pub addr: u64,
    pub len: u64,
}

// #[cfg(target_os = "none")]
// #[cfg(not(test))]
pub static RAMDISK_INFO: Mutex<Option<RamdiskInfo>> = Mutex::new(None);
// #[cfg(not(target_os = "none"))]
// #[cfg(test)]
// pub static RAMDISK_INFO: std::sync::Mutex<Option<RamdiskInfo>> = std::sync::Mutex::new(None);

#[derive(thiserror::Error, Debug)]
pub enum InitError {
    #[error("Failed to setup heap because {0:?}")]
    FailedToSetupHeap(MapToError<Size4KiB>),
    #[error("Failed to setup system timer")]
    FailedToSetupSystemTimer(#[from] SystemTimerError),
    #[error("Failed to setup the kernel logger")]
    FailedToSetupLogger,
}

/// # Errors
/// Will return [`InitError::FailedToSetupHeap`] if it could not map the pages for the heap.
/// Will also return [`InitError::FailedToSetupSystemTimer`] if the system timer was already owned
/// # Panics
/// Will panic if no physical memory offset could be found
pub fn kernel_early(
    boot_info: &'static mut BootInfo,
    frequency: pit::PitFrequency,
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
    #[cfg(target_os = "none")]
    allocator::setup_heap(&mut mapper, &mut frame_allocator)
        .map_err(InitError::FailedToSetupHeap)?;

    kernel_logger::init(logger::store).or(Err(InitError::FailedToSetupLogger))?;

    info!("Heap setup, can start logging");

    if let Some(framebuffer) = boot_info.framebuffer.take() {
        framebuffer::init(framebuffer);
        info!("Initialized framebuffer");
    }

    gdt::init();
    info!("The GDT was initialized");

    interrupts::init_idt();
    info!("The IDT was initialized");

    unsafe { interrupts::PICS.acquire().initialize() };
    info!("The PIC was initialized");
    interrupts::unmask();
    info!("Unmasked interrupts");

    x86_64::instructions::interrupts::enable();
    info!("Interrupts Enabled");

    timer::setup_system_timer(frequency)?;
    info!("System timer Initialized");

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
