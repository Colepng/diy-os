#![no_std]
#![no_main]
#![feature(optimize_attribute)]
#![feature(custom_test_frameworks)]
#![feature(never_type)]
#![feature(pointer_is_aligned_to)]
#![feature(iter_collect_into)]
#![feature(sync_unsafe_cell)]
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
#![allow(clippy::inline_always)]

extern crate alloc;

use alloc::{boxed::Box, string::String};
use bootloader_api::{
    BootInfo, BootloaderConfig,
    config::{Mapping, Mappings},
    entry_point,
};
use core::panic::PanicInfo;
use diy_os::{
    filesystem::ustar,
    hlt_loop,
    human_input_devices::STDIN,
    kernel_early,
    log::{self, LogLevel},
    multitasking::Task,
    println,
    ps2::{controller::PS2Controller, devices::keyboard::Keyboard, GenericPS2Controller},
    timer::{sleep, TIME_KEEPER},
};
use diy_os::{
    human_input_devices::process_keys,
    log::trace,
    multitasking::{CURRENT_TASK, schedule},
    ps2::devices::ps2_device_1_task,
};
use x86_64::VirtAddr;
use x86_64::structures::paging::FrameAllocator;
use x86_64::structures::paging::Mapper;
use x86_64::structures::paging::Page;
use x86_64::structures::paging::Size4KiB;
static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    let mut mappings = Mappings::new_default();
    mappings.physical_memory = Some(Mapping::Dynamic);
    config.mappings = mappings;

    config
};

entry_point!(main_wrapper, config = &BOOTLOADER_CONFIG);

// SAFETY: there is no other global function of this name
#[unsafe(no_mangle)]
extern "Rust" fn main_wrapper(boot_info: &'static mut BootInfo) -> ! {
    match main(boot_info) {
        Err(err) => panic!("{err:?}"),
    }
}

// SAFETY: there is no other global function of this name
#[unsafe(no_mangle)]
extern "Rust" fn main(boot_info: &'static mut BootInfo) -> anyhow::Result<!> {
    let (boot_info, mut frame_allocator, mut mapper) = kernel_early(boot_info, 1000)?;

    let ramdisk_addr = boot_info.ramdisk_addr.into_option().unwrap();
    let _ramdisk = unsafe { ustar::Ustar::new(ramdisk_addr.try_into()?) };

    println!("Hello, world!");

    let gernaric = GenericPS2Controller::new();

    let gernaric = gernaric.initialize();

    {
        diy_os::ps2::CONTROLLER.acquire().replace(gernaric);
        diy_os::ps2::PS1_DEVICE
            .acquire()
            .replace(Box::new(Keyboard::new()));
    }

    setup_tasks(&mut mapper, &mut frame_allocator);
}

fn setup_tasks(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> ! {
    let current_stack = rsp();
    let current_task = Box::leak(Task::allocate_task(
        String::from("Main Task"),
        0,
        Page::<Size4KiB>::containing_address(VirtAddr::new(current_stack)).start_address() + 0x1000,
        VirtAddr::new(current_stack),
    ));

    current_task.next = core::ptr::from_mut(current_task);

    CURRENT_TASK.with_mut_ref(|task| {
        task.replace(current_task);
    });

    // # SAFETY: ps2_device_1_task calles schedule once per loop
    unsafe { Task::new(String::from("PS/2 Deivce 1 Task"), ps2_device_1_task, mapper, frame_allocator).unwrap() };
    // # SAFETY: process_keys calles schedule once per loop
    unsafe { Task::new(String::from("Proccess keys"), process_keys, mapper, frame_allocator).unwrap() };
    // # SAFETY: kernal_shell calles schedule once per loop
    unsafe { Task::new(String::from("Kernal Shell"), kernal_shell, mapper, frame_allocator).unwrap() };

    TIME_KEEPER.with_mut_ref(|keeper| keeper.schedule_counter.time.reset());

    loop {
        schedule();
    }
}

fn kernal_shell() -> ! {
    let mut input = String::new();

    loop {
        STDIN.with_mut_ref(|stdin| {
            stdin
                .drain(..stdin.len())
                .map(|keycode| {
                    let char = char::from(keycode);
                    diy_os::print!("{char}");
                    char
                })
                .collect_into(&mut input);
        });

        if input.contains('\n') {
            let lines = input.lines();

            for line in lines {
                let mut words = line.split_whitespace();
                if let Some(first_word) = words.next() {
                    match first_word {
                        "SLEEP" => {
                            if let Some(word) = words.next() {
                                let result = word.parse();

                                if let Ok(amount) = result {
                                    trace(alloc::format!("sleeping for {amount}").leak());
                                    sleep(amount);
                                    trace(alloc::format!("done sleeping for {amount}").leak());
                                    println!("done sleeping");
                                } else {
                                    println!("pls input a number");
                                }
                            }
                        }
                        "PANIC" => {
                            panic!("yo fuck you no more os");
                        }
                        "LOGS" => {
                            let log_level =
                                words.next().map_or(LogLevel::Debug, |level| match level {
                                    "ERROR" => LogLevel::Error,
                                    "WARN" => LogLevel::Warn,
                                    "INFO" => LogLevel::Info,
                                    "DEBUG" => LogLevel::Debug,
                                    "TRACE" => LogLevel::Trace,
                                    _ => {
                                        println!("Invalid log level, defauting to debug");
                                        LogLevel::Debug
                                    }
                                });

                            log::LOGGER.with_ref(|logger| {
                                logger
                                    .get_events()
                                    .filter(|event| event.level <= log_level)
                                    .for_each(|event| println!("{}", event))
                            });
                        }
                        command => println!("{command} is invalid"),
                    }
                }
            }

            input.clear();
        }

        schedule();
    }
}

#[inline(always)]
fn rsp() -> u64 {
    let rsp: u64;

    unsafe { core::arch::asm!("mov {stack}, rsp", stack = out(reg) rsp) }

    rsp
}

/// This function is called on panic.
#[cfg(not(test))] // new attribute
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    log::LOGGER.with_ref(|logger| logger.get_events().for_each(|event| println!("{}", event)));
    hlt_loop();
}
