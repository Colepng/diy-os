#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![feature(never_type)]
#![feature(iter_collect_into)]
#![feature(const_trait_impl)]
#![feature(const_convert)]
#![test_runner(diy_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use zerocopy::FromBytes;
extern crate alloc;

use alloc::{boxed::Box, string::String, sync::Arc};
use bootloader_api::{
    BootInfo, BootloaderConfig,
    config::{Mapping, Mappings},
    entry_point,
};
use core::panic::PanicInfo;
use diy_os::{
    device_manager::{self, BlockDevice},
    filesystem::{
        FileSystem, FileSystemSetupError, VFS,
        gpt::{self, PartionTableHeader, PartitionEntry},
    },
    human_input_devices::{STDIN, process_keys},
    kernel_early,
    multitasking::{SCHEDULER, Task, mutex::Mutex, sleep},
    pit::PitFrequency,
    print, println,
    ps2::devices::ps2_device_1_task,
    timer::{Duration, Miliseconds, Seconds, TIME_KEEPER},
};
use fat16_read_only::fat_setup;
use log::{Level, info, trace};
use qemu_exit::QEMUExit;
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
        Err(err) => panic!("{err:#?}"),
    }
}

// SAFETY: there is no other global function of this name
#[unsafe(no_mangle)]
extern "Rust" fn main(boot_info: &'static mut BootInfo) -> anyhow::Result<!> {
    let frequency = refine_const!(1000u32, PitFrequency);
    let (_boot_info, mut frame_allocator, mut mapper) = kernel_early(boot_info, frequency)?;

    info!("start_address {:X}", 0x0000_0000_0804_aff8);
    info!("start_address {:X}", 0x0000_0000_0804_aff8 + 4000 * 3);
    info!("allocater start {:?}", diy_os::allocator::HEAP_START);
    info!("allocater end {:?}", unsafe {
        diy_os::allocator::HEAP_START.byte_add(diy_os::allocator::HEAP_SIZE)
    });

    println!("Hello, world!");

    let device_manager = device_manager::init_device_manager()?;

    device_manager.print_devices();

    // hardcoded for now
    let device = device_manager.block_devices[1].clone();

    let fs = setup_filesystem(&device)?;

    let mut vfs = VFS::new(fs);

    let file = vfs.open("/door/ads.txt").unwrap();

    let mut buf = [0u8; 100];

    let _ = file.read(&mut buf).unwrap();

    let text = str::from_utf8(&buf)?;

    println!("text of a.txt: {text:?}");
    //
    // panic!("exit");

    // for i in 0..=255 {
    //     for j in 0..32 {
    //         if let Some(dev_info) = diy_os::pci::get_info(i, j, 0) {
    //             if dev_info.header_type.multi_func() {
    //                 for k in 1..=7 {
    //                     let device = diy_os::pci::get_info(i, j, k);
    //                     println!("dev_info func {k}: {device:#X?}");
    //                 }
    //             }
    //             println!("dev_info: {dev_info:#X?}");
    //         }
    //     }
    // }

    // let's just say we detected that bus 0 device 1 function 1 was the ide controller

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

    // let fat32_driver = Task::new(
    //     String::from("fat32 driver"),
    //     wrapper,
    //     mapper,
    //     frame_allocator,
    // );

    // let ide = Task::new(String::from("ide"), ide_task, mapper, frame_allocator);

    let (_ps2_task, _keys_task, _shell_task) = SCHEDULER.with_mut_ref(|scheduler| {
        let ps2_task = scheduler.spawn_task(ps2_task);
        let keys_task = scheduler.spawn_task(keys_task);
        let shell_task = scheduler.spawn_task(shell_task);
        // let _ = scheduler.spawn_task(fat32_driver);
        // let _ = scheduler.spawn_task(ide);

        (ps2_task, keys_task, shell_task)
    });

    loop {
        sleep(Seconds(1).into());
        // debug!("Main task is still running properly");
    }
}

// fn testing_task() -> ! {}

fn kernal_shell() -> ! {
    let mut input = String::new();

    print!(">>");

    loop {
        sleep(Duration::from(Miliseconds(10)));
        STDIN.with_mut_ref(|stdin| {
            stdin
                .drain(..stdin.len())
                .filter_map(|keycode| {
                    char::try_from(keycode).map_or(None, |char| {
                        diy_os::print!("{char}");
                        Some(char)
                    })
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
                        "QUIT" | "EXIT" => {
                            let exit_handle = qemu_exit::X86::new(0xf4, 3);

                            exit_handle.exit_success();
                        }
                        command => println!("{command} is invalid"),
                    }
                }

                print!(">>");
            }

            input.clear();
        }
    }
}

#[allow(clippy::inline_always)]
#[inline(always)]
fn rsp() -> u64 {
    let rsp: u64;

    unsafe { core::arch::asm!("mov {stack}, rsp", stack = out(reg) rsp) }

    rsp
}
/// Reads the GPT from `device`, parses the partition table, and mounts the
/// filesystem on the first partition.
///
/// Returns a boxed [`FileSystem`] for the mounted partition.
///
/// # Errors
///
/// Returns [`FileSystemSetupError`] if:
/// - the GPT header is missing, corrupt, or fails CRC validation
/// - the device read fails
/// - the partition's filesystem driver fails to mount
///
pub fn setup_filesystem(
    device: &Arc<Mutex<dyn BlockDevice>>,
) -> Result<Box<dyn FileSystem>, FileSystemSetupError> {
    let header = PartionTableHeader::from_device(device)?;

    // Currently only 128 sized partition entries are implemented
    assert_eq!(128, header.size_of_partion_entry);

    let array_size = usize::try_from(header.size_of_partion_entry)
        .unwrap()
        .checked_mul(usize::try_from(header.num_of_partions).unwrap())
        .unwrap();

    let mut buffer = alloc::vec![0u8; array_size];

    {
        let mut drive = device.acquire();

        let sector_size = drive.sector_size();

        drive.read_sectors(
            header.partion_entry_lba,
            u8::try_from(array_size.div_ceil(sector_size)).unwrap(),
            &mut buffer,
        )?;
    }

    let entries = <[PartitionEntry]>::ref_from_bytes(&buffer).unwrap();

    for partion in entries {
        if partion.partion_type_guid.get() != 0 {
            let name = partion.name().unwrap();

            crate::println!("partion {name}, partion {:?}", partion);
        }
    }

    match entries[0].get_fs().unwrap() {
        gpt::FSGuid::SimpleFileSystem => todo!(),
        gpt::FSGuid::MicrosoftData => Ok(fat_setup(device.clone(), &entries[0])?),
        _ => todo!(),
    }
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

    let exit_handle = qemu_exit::X86::new(0xf4, 3);

    exit_handle.exit_failure();
}
