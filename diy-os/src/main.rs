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
#![allow(clippy::inline_always, clippy::linkedlist)]

extern crate alloc;

use alloc::string::String;
use bootloader_api::{
    BootInfo, BootloaderConfig,
    config::{Mapping, Mappings},
    entry_point,
};
use core::panic::PanicInfo;
use diy_os::{
    filesystem::ustar,
    hlt_loop,
    human_input_devices::{process_keys, STDIN},
    kernel_early,
    multitasking::{exit, mutex::Mutex, sleep, Task, SCHEDULER},
    pit::PitFrequency,
    print, println,
    ps2::devices::ps2_device_1_task,
    timer::{Duration, Miliseconds, Seconds, TIME_KEEPER},
};
use log::{Level, debug, trace};
use refine::Refined;
use refine::refine_const;
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

    setup_tasks(&mut mapper, &mut frame_allocator)?;
}

fn setup_tasks(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> anyhow::Result<!> {
    let current_stack = rsp();
    let current_task = Task::allocate_task(
        String::from("Main Task"),
        Page::<Size4KiB>::containing_address(VirtAddr::new(current_stack)).start_address() + 0x1000,
        VirtAddr::new(current_stack),
    );

    SCHEDULER.with_mut_ref(|scheduler| {
        scheduler.set_first_task(current_task);
        scheduler.setup_special_tasks(mapper, frame_allocator);
    });

    TIME_KEEPER.with_mut_ref(|keeper| keeper.schedule_counter.time.reset());
    // main task starts here
    // # SAFETY: ps2_device_1_task calls schedule once per loop
    let ps2_task = Task::new(
        String::from("PS/2 Deivce 1 Task"),
        ps2_device_1_task,
        mapper,
        frame_allocator,
    );

    // # SAFETY: process_keys calls schedule once per loop
    let keys_task = Task::new(
        String::from("Proccess keys"),
        process_keys,
        mapper,
        frame_allocator,
    );

    // # SAFETY: kernal_shell calle schedule once per loop
    let shell_task = Task::new(
        String::from("Kernal Shell"),
        kernal_shell,
        mapper,
        frame_allocator,
    );

    let blocked_task = Task::new(
        String::from("Blocked Task"),
        to_be_slept,
        mapper,
        frame_allocator,
    );

    let dead_task = Task::new(
        String::from("im gonna die"),
        to_be_killed,
        mapper,
        frame_allocator,
    );

    let counter_1 = Task::new(
        String::from("counter 1"),
        counter_1,
        mapper,
        frame_allocator,
    );

    let counter_2 = Task::new(
        String::from("counter 2"),
        counter_2,
        mapper,
        frame_allocator,
    );

    let (_ps2_task, _keys_task, _shell_task, _blocked_task) = SCHEDULER.with_mut_ref(|scheduler| {
        let ps2_task = scheduler.spawn_task(ps2_task);
        let keys_task = scheduler.spawn_task(keys_task);
        let shell_task = scheduler.spawn_task(shell_task);
        let blocked_task = scheduler.spawn_task(blocked_task);
        let _ = scheduler.spawn_task(dead_task);
        let _ = scheduler.spawn_task(counter_1);
        let _ = scheduler.spawn_task(counter_2);

        (ps2_task, keys_task, shell_task, blocked_task)
    });

    loop {
        sleep(Seconds(1).into());
        debug!("Main task is still running properly");
        println!("Main task is still running properly");
    }
}

fn to_be_slept() -> ! {
    loop {
        println!("not sleeping");
        sleep(Duration::from(Seconds(10)));
    }
}

fn to_be_killed() -> ! {
    println!("im gonna die now");

    sleep(Seconds(10).into());

    unsafe { exit() };
}

static COUNTER: Mutex<u8> = Mutex::new(0);

fn counter_1() -> ! {
    println!("getting mutex");
    let mut counter = COUNTER.acquire();
    *counter += 1;
    sleep(Seconds(1).into());
    drop(counter);
    println!("released mutex");

    unsafe { exit() }
}

fn counter_2() -> ! {
    println!("trying to get mutex");
    let counter = COUNTER.acquire();
    println!("counter: {}", counter);
    drop(counter);
    println!("droped counter");

    unsafe { exit() }
}

fn kernal_shell() -> ! {
    let mut input = String::new();

    loop {
        sleep(Duration::from(Miliseconds(10)));
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
                                    sleep(Miliseconds(amount).into());
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
                        "SCHED" => {
                            SCHEDULER.with_ref(|sched| {
                                sched.print_state();
                            });
                            TIME_KEEPER.with_ref(|time_keeper| {
                                println!("time_since_boot: {}", time_keeper.time_since_boot.time);
                            });
                        }
                        command => println!("{command} is invalid"),
                    }
                }
            }

            input.clear();
        }
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
    use diy_os::framebuffer;

    if framebuffer::FRAME_BUFER.is_acquired() {
        use diy_os::serial;

        framebuffer::FRAME_BUFER.release();
        serial::SERIAL1.release();

        print!("forced release");
    }

    println!("{}", info);
    if diy_os::logger::LOGGER.is_acquired() {
        println!("logger was locked, cracking it open");

        diy_os::logger::LOGGER.release();
    }

    diy_os::logger::LOGGER.with_ref(|logger| {
        logger.get_events().for_each(|event| println!("{}", event));
    });

    if SCHEDULER.is_acquired() {
        println!("scheduler was locked");

        println!("forcing open");

        SCHEDULER.release();
    }

    SCHEDULER.with_ref(|sched| {
        sched.print_state();
    });

    hlt_loop();
}
