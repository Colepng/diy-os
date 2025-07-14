#![no_std]
#![no_main]
#![feature(optimize_attribute)]
#![feature(custom_test_frameworks)]
#![feature(never_type)]
#![feature(pointer_is_aligned_to)]
#![feature(iter_collect_into)]
#![feature(sync_unsafe_cell)]
#![feature(const_trait_impl)]
#![feature(const_from)]
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
use refine::refine_const;
use refine::Refined;
use core::panic::PanicInfo;
use diy_os::{
    filesystem::ustar, hlt_loop, human_input_devices::{process_keys, STDIN}, kernel_early, multitasking::{schedule, Task, SCHEDULER}, pit::PitFrequency, println, ps2::{
        controller::PS2Controller, devices::{keyboard::Keyboard, ps2_device_1_task}, GenericPS2Controller, CONTROLLER, PS1_DEVICE
    }, timer::{sleep, TIME_KEEPER}
};
use log::{Level, trace};
use x86_64::{
    VirtAddr,
    structures::paging::{FrameAllocator, Mapper, Page, Size4KiB},
};
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
    let frequency = refine_const!(1000u32, PitFrequency);
    let (boot_info, mut frame_allocator, mut mapper) = kernel_early(boot_info, frequency)?;

    let _ramdisk = if let Some(addr) = boot_info.ramdisk_addr.into_option() {
        Some(unsafe { ustar::Ustar::new(addr.try_into()?) })
    } else {
        None
    };

    println!("Hello, world!");

    let gernaric = GenericPS2Controller::new();

    let gernaric = gernaric.initialize();

    CONTROLLER.with_mut_ref(|controller| controller.replace(gernaric));
    PS1_DEVICE.with_mut_ref(|ps1| ps1.replace(Box::new(Keyboard::new())));

    setup_tasks(&mut mapper, &mut frame_allocator);
}

fn setup_tasks(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> ! {
    let current_stack = rsp();
    let current_task = Task::allocate_task(
        String::from("Main Task"),
        0,
        Page::<Size4KiB>::containing_address(VirtAddr::new(current_stack)).start_address() + 0x1000,
        VirtAddr::new(current_stack),
    );

    SCHEDULER.with_mut_ref(|scheduler| {
        scheduler.set_first_task(current_task);
    });

    // # SAFETY: ps2_device_1_task calles schedule once per loop
    unsafe {
        Task::new(
            String::from("PS/2 Deivce 1 Task"),
            ps2_device_1_task,
            mapper,
            frame_allocator,
        )
        .unwrap();
    };

    // # SAFETY: process_keys calles schedule once per loop
    unsafe {
        Task::new(
            String::from("Proccess keys"),
            process_keys,
            mapper,
            frame_allocator,
        )
        .unwrap();
    };

    // # SAFETY: kernal_shell calles schedule once per loop
    unsafe {
        Task::new(
            String::from("Kernal Shell"),
            kernal_shell,
            mapper,
            frame_allocator,
        )
        .unwrap();
    };

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
                                    trace!("sleeping for {amount}");
                                    sleep(amount);
                                    trace!("done sleeping for {amount}");
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
                                words.next().map_or(Level::Debug, |level| match level {
                                    "ERROR" => Level::Error,
                                    "WARN" => Level::Warn,
                                    "INFO" => Level::Info,
                                    "DEBUG" => Level::Debug,
                                    "TRACE" => Level::Trace,
                                    _ => {
                                        println!("Invalid log level, defauting to debug");
                                        Level::Debug
                                    }
                                });

                            diy_os::logger::LOGGER.with_ref(|logger| {
                                logger
                                    .get_events()
                                    .filter(|event| event.level <= log_level)
                                    .for_each(|event| println!("{}", event));
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
    diy_os::logger::LOGGER
        .with_ref(|logger| logger.get_events().for_each(|event| println!("{}", event)));

    if let Some(scheduler) = SCHEDULER.try_acquire() {
        let task = scheduler.get_current_task();
        println!("{:#?}", task);
    } else {
        println!("scheduler was locked");
    }

    hlt_loop();
}
