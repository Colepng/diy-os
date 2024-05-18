#![no_std]
#![no_main]
#![feature(optimize_attribute)]
#![feature(custom_test_frameworks)]
#![feature(f16)]
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
    pit::{self, AccessMode, Pit, ReadBackCommand},
    println,
};
use x86_64::instructions::hlt;

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

    let mut pit = pit::Pit::new();
    let read_back_command = pit::ReadBackCommandBuilder::new()
        .set_read_from_channel_0(true)
        .set_read_status_byte(true)
        .build();

    let configure_command = pit::ConfigureChannelCommand::new(
        pit::Channel::Channel0,
        AccessMode::LowHighbyte,
        pit::OperatingMode::SquareWaveGenerator,
        pit::BcdBinaryMode::Binary16Bit,
    );

    pit.mode_port.write(configure_command);

    // pit.mode_port.write(read_back_command);
    //
    let reaload_value = get_reload_value_from_frequency(1000);

    println!("readlaod vlaue {}", reaload_value);

    set_count(&mut pit, reaload_value);

    println!("going to sleep");

    sleep(1000);

    println!("wakign up");

    hlt_loop();
}

pub fn get_reload_value_from_frequency(frequency: u32) -> u16 {
    u16::try_from(1192182 / frequency).unwrap()
}

pub fn set_count(pit: &mut Pit, count: u16) -> &mut Pit {
    x86_64::instructions::interrupts::without_interrupts(|| {
        pit.channel_0_port.write((count & 0xFF).try_into().unwrap()); // low_byte
        pit.channel_0_port
            .write(((count & 0xFF00) >> 8).try_into().unwrap()) // high byte
    });

    pit
}

// time in ms
pub fn sleep(count: u64) {
    *pit::SLEEP_COUNTER.acquire() = count;
    pit::SLEEP_COUNTER.release();
    
    while *pit::SLEEP_COUNTER.acquire() > 0 {
        pit::SLEEP_COUNTER.release();
        hlt();
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
