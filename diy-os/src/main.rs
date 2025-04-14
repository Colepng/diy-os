#![no_std]
#![no_main]
#![feature(optimize_attribute)]
#![feature(custom_test_frameworks)]
#![feature(naked_functions)]
#![feature(never_type)]
#![feature(pointer_is_aligned_to)]
#![feature(iter_collect_into)]
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

use alloc::{boxed::Box, string::String};
use bootloader_api::{
    BootInfo, BootloaderConfig,
    config::{Mapping, Mappings},
    entry_point,
};

use core::panic::PanicInfo;
use diy_os::{
    elf, filesystem::ustar, hlt_loop, human_input_devices::{ProccesKeys, STDIN}, kernel_early, log::{self, LogLevel}, multitasking::TaskRunner, println, ps2::{
        controller::PS2Controller, devices::{keyboard::Keyboard, PS2Device1Task}, GenericPS2Controller
    }, timer::sleep
};
use x86_64::structures::paging::{FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB};

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
    let (boot_info, _frame_allocator, _mapper) = kernel_early(boot_info, 1000)?;

    let ramdisk_addr = boot_info.ramdisk_addr.into_option().unwrap();
    let ramdisk = unsafe { ustar::Ustar::new(ramdisk_addr.try_into()?) };

    println!("Hello, world!");

    // let elf_file = &ramdisk.get_files()[0];

    let gernaric = GenericPS2Controller::new();

    let gernaric = gernaric.initialize();

    {
        diy_os::ps2::CONTROLLER.acquire().replace(gernaric);
        diy_os::ps2::PS1_DEVICE
            .acquire()
            .replace(Box::new(Keyboard::new()));
    }

    // let _ = load_elf_and_jump_into_it(elf_file, &mut mapper, &mut frame_allocator);

    // hlt_loop();
    let mut task_runner = TaskRunner::new();

    task_runner.add_task(PS2Device1Task);
    task_runner.add_task(ProccesKeys);
    task_runner.add_task(KernelShell::new());

    task_runner.start_running();
}

struct KernelShell {
    input: String,
}

impl KernelShell {
    pub fn new() -> Self {
        Self {
            input: String::new(),
        }
    }
}

impl diy_os::multitasking::Task for KernelShell {
    fn run(&mut self) {
        STDIN.with_mut_ref(|stdin| {
            stdin
                .drain(..stdin.len())
                .filter_map(|keycode| {
                    let char = char::from(keycode);
                    diy_os::print!("{char}");
                    Some(char)
                })
                .collect_into(&mut self.input);
        });

        if self.input.contains('\n') {
            let lines = self.input.lines();

            for line in lines {
                let mut words = line.split_whitespace();
                if let Some(first_word) = words.next() {
                    match first_word {
                        "SLEEP" => {
                            if let Some(word) = words.next() {
                                let result = word.parse();

                                if let Ok(amount) = result {
                                    log::trace(alloc::format!("sleeping for {}", amount).leak());
                                    sleep(amount);
                                    log::trace(alloc::format!("done sleeping for {}", amount).leak());
                                    println!("done sleeping");
                                } else {
                                    println!("pls input a number")
                                }
                            }

                        }
                        "PANIC" => {
                            panic!("yo fuck you no more os");
                        }
                        "LOGS" => {
                            let log_level = if let Some(level) = words.next() {
                                match level {
                                    "ERROR" => LogLevel::Error,
                                    "WARN" => LogLevel::Warn,
                                    "INFO" => LogLevel::Info,
                                    "DEBUG" => LogLevel::Debug,
                                    "TRACE" => LogLevel::Trace,
                                    _ => {
                                        println!("Invalid log level, defauting to debug");
                                        LogLevel::Debug
                                    },
                                }
                            } else {
                                LogLevel::Debug
                            };

                            log::LOGGER.with_ref(|logger| logger.get_events().filter(|event| event.level <= log_level).for_each(|event| println!("{}", event)));
                        }
                        command => println!("{command} is invalid"),
                    }
                }
            }

            self.input.clear();
        }
    }
}

#[repr(u8)]
enum ExitCode {
    Successful = 0,
}

fn load_elf_and_jump_into_it(
    file: &ustar::File,
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<ExitCode, u8> {
    let file_ptr = file.get_raw_bytes().unwrap().as_ptr();
    let elf_header = unsafe { &*file_ptr.cast::<elf::Header>() };

    println!("header, {:#?}", elf_header);

    let program_header = unsafe {
        &*file_ptr
            .byte_offset(elf_header.program_header_table_offset as isize)
            .cast::<elf::ProgramHeaderTableEntry>()
    };

    println!("program_header: {:#?}", program_header);

    let ph_virtaddr = program_header.virtual_address;

    println!(
        "check alignment {}",
        program_header.virtual_address.as_u64() % program_header.alignment
    );

    let page_for_load: Page<Size4KiB> = Page::containing_address(ph_virtaddr);

    // Has to be writable to write the load segment to the page in the first place
    let flags = PageTableFlags::USER_ACCESSIBLE
        | PageTableFlags::WRITABLE
        | PageTableFlags::PRESENT
        | PageTableFlags::NO_CACHE; //| PageTableFlags::
    //
    let frame = frame_allocator.allocate_frame().unwrap();

    // Setup page for load
    unsafe {
        mapper
            .map_to_with_table_flags(page_for_load, frame, flags, flags, frame_allocator)
            .unwrap()
            .flush();
    }

    unsafe {
        // Zeros mem for instructions
        ph_virtaddr
            .as_mut_ptr::<u8>()
            .write_bytes(0, program_header.size_of_segment_mem as usize);

        file_ptr
            .add(elf_header.program_header_table_offset)
            .add(elf_header.program_header_entry_size as usize)
            .copy_to_nonoverlapping(
                ph_virtaddr.as_mut_ptr::<u8>(),
                program_header.size_of_segment_file as usize,
            );
    }

    // Setup page for stack
    let page_stack: Page<Size4KiB> = Page::containing_address(ph_virtaddr + 0x2000);
    let stack_flags = PageTableFlags::USER_ACCESSIBLE
        | PageTableFlags::WRITABLE
        | PageTableFlags::PRESENT
        | PageTableFlags::NO_CACHE;
    let stack_frame = frame_allocator.allocate_frame().unwrap();

    // map page for stack
    unsafe {
        mapper
            .map_to_with_table_flags(
                page_stack,
                stack_frame,
                stack_flags,
                stack_flags,
                frame_allocator,
            )
            .unwrap()
            .flush();
    }

    diy_os::usermode::into_usermode(
        program_header.virtual_address.as_u64(),
        page_stack.start_address().as_u64() + 0x1000,
    );

    Ok(ExitCode::Successful)
}

/// This function is called on panic.
#[cfg(not(test))] // new attribute
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    hlt_loop();
}

// #[cfg(test)]
// #[panic_handler]
// fn panic(info: &PanicInfo) -> ! {
//     diy_os::test_panic_handler(info)
// }

// test to make sure tests won't panic
// #[test_case]
// fn trivial_assertion() {
//     assert_eq!(1, 1);
// }
