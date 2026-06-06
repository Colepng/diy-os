#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]
#![feature(abi_x86_interrupt)]
#![feature(anonymous_lifetime_in_impl_trait)]
#![feature(str_from_raw_parts)]
#![feature(transmutability)]
#![feature(strict_provenance_lints)]
#![feature(variant_count)]
#![feature(const_trait_impl)]
#![feature(const_convert)]
#![feature(ascii_char)]
#![feature(ascii_char_variants)]
#![feature(int_from_ascii)]
#![feature(test)] // clippy can't check if test is needed
#![feature(slice_ptr_get)]
#![feature(iter_array_chunks)]
#![deny(fuzzy_provenance_casts)]

use bootloader_api::BootInfo;
use log::info;
use memory::BootInfoFrameAllocator;
use timer::SystemTimerError;
use x86_64::structures::paging::{OffsetPageTable, Size4KiB, mapper::MapToError};

extern crate alloc;

#[cfg(not(test))]
pub mod allocator;
pub mod collections;
pub mod console;
pub mod device_manager;
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
pub mod pci;
pub mod pit;
pub mod ps2;
pub mod serial;
pub mod syscalls;
pub mod timer;
pub mod usermode;

pub const P_OFFSET: u64 = 0xffff800000000000;

#[derive(Debug, Clone, Copy)]
pub struct RamdiskInfo {
    pub addr: u64,
    pub len: u64,
}

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
    #[cfg(not(test))]
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
