#![no_std]
#![no_main]
#![feature(optimize_attribute)]
#![feature(custom_test_frameworks)]
#![feature(never_type)]
#![feature(pointer_is_aligned_to)]
#![feature(iter_collect_into)]
#![feature(sync_unsafe_cell)]
#![feature(const_trait_impl)]
#![feature(const_convert)]
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

use alloc::{boxed::Box, string::String};
use bootloader_api::{
    BootInfo, BootloaderConfig,
    config::{Mapping, Mappings},
    entry_point,
};
use core::{panic::PanicInfo, ptr};
use diy_os::{
    P_OFFSET, RamdiskInfo,
    allocator::{HEAP_SIZE, HEAP_START},
    human_input_devices::{STDIN, process_keys},
    kernel_early,
    memory::{self, BootInfoFrameAllocator, PMM},
    multitasking::{SCHEDULER, Task, sleep},
    pit::PitFrequency,
    print, println,
    ps2::devices::ps2_device_1_task,
    timer::{Duration, Miliseconds, Seconds, TIME_KEEPER},
};
use log::{Level, info, trace};
use qemu_exit::QEMUExit;
use refine::Refined;
use refine::refine_const;
use x86_64::{
    PhysAddr, VirtAddr,
    registers::control::Cr3,
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTableFlags, PhysFrame, Size4KiB,
        Translate, mapper::CleanUp,
    },
};

static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    let mut mappings = Mappings::new_default();
    mappings.physical_memory = Some(Mapping::FixedAddress(P_OFFSET));
    // 64 TB mapping ends at 0xffffc00000000000 - 1
    // another tb for continuous memory, for now only heap at 0xffffc00000000000
    // another tb for quick and dirty stacks at 0xffffc10000000000
    // the rest will be free to the virtual memory allacator
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

fn map_addr_range(
    start_addr: u64,
    end_addr: u64,
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    flags: PageTableFlags,
) {
    for paddr in (start_addr..end_addr).step_by(ADDR) {
        let vaddr = VirtAddr::new(0xffff800000000000 + paddr);
        let pframe = PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(paddr));
        let page = Page::containing_address(vaddr);

        unsafe {
            mapper.map_to(page, pframe, flags, frame_allocator).unwrap();
        }
    }
}

// SAFETY: there is no other global function of this name
#[unsafe(no_mangle)]
extern "Rust" fn main(boot_info: &'static mut BootInfo) -> anyhow::Result<!> {
    let frequency = refine_const!(1000u32, PitFrequency);
    let (boot_info, mut frame_allocator, mut mapper) = kernel_early(boot_info, frequency)?;

    info!("kernel vaddr: {:X}", boot_info.kernel_image_offset);
    info!("kernel size: {:X}", boot_info.kernel_len);
    info!("start_address {:X}", 0x0000_0000_0804_aff8);
    info!("start_address {:X}", 0x0000_0000_0804_aff8 + 4000 * 3);
    // info!("allocater start {:?}", diy_os::allocator::HEAP_START);
    // info!("allocater end {:?}", unsafe {
    //     diy_os::allocator::HEAP_START.byte_add(diy_os::allocator::HEAP_SIZE)
    // });
    //
    //
    // what memory does the kernel need
    // - code/binary
    // - stack (handled by the tss)
    // - heap for data structures
    // - framebuffer for io

    let mut level_4_page_table = Box::new(mapper.level_4_table().clone());
    let map = unsafe {
        OffsetPageTable::new(
            level_4_page_table.as_mut(),
            VirtAddr::new(0xffff800000000000),
        )
    };

    // since the keneral only has ADDR00000 mem, map from addres 0 to 0x100000000
    // let mut map = diy_os::memory::setup_virtual_memory_map(
    //     &mut frame_allocator,
    //     VirtAddr::new(boot_info.physical_memory_offset.into_option().unwrap()),
    // );
    //
    // let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::GLOBAL;
    //
    // // map phyc memory at an offset
    // map_addr_range(0x0, ADDR00000, &mut map, &mut frame_allocator, flags);
    //
    // // map kernel code to address space
    // let kernel_paddr = boot_info.kernel_addr;
    // let size = boot_info.kernel_len;
    // let flags = PageTableFlags::PRESENT | PageTableFlags::GLOBAL;
    //
    // map_addr_range(
    //     kernel_paddr,
    //     kernel_paddr + size,
    //     &mut map,
    //     &mut frame_allocator,
    //     flags,
    // );
    //
    // // maps heap
    // let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::GLOBAL;
    // map_addr_range(
    //     HEAP_START,
    //     HEAP_START + HEAP_SIZE,
    //     &mut map,
    //     &mut frame_allocator,
    //     flags,
    // );
    //
    // // map framebuffer
    //
    // map_addr_range(start_addr, end_addr, mapper, frame_allocator, flags);

    if let Some(addr) = boot_info.ramdisk_addr.into_option() {
        info!("ramdisk start {addr:X}");
        info!("ramdisk end {:X}", addr + boot_info.ramdisk_len);

        let ramdisk_info = RamdiskInfo {
            addr,
            len: boot_info.ramdisk_len,
        };

        diy_os::RAMDISK_INFO.with_mut_ref(|info| info.replace(ramdisk_info));
    }

    println!("Hello, world!");

    let cr3 = Cr3::read().0.start_address().as_u64();

    println!("cr3: {:X}", cr3);

    setup_tasks(&mut mapper, frame_allocator)?;
}

fn setup_tasks(
    mapper: &mut (impl Mapper<Size4KiB> + CleanUp),
    mut frame_allocator: BootInfoFrameAllocator,
) -> anyhow::Result<!> {
    let current_stack = rsp();
    let current_task = Task::allocate_task(
        String::from("Main Task"),
        Page::<Size4KiB>::containing_address(VirtAddr::new(current_stack)).start_address() + 0x1000,
        VirtAddr::new(current_stack),
        &mut frame_allocator,
    );

    SCHEDULER.with_mut_ref(|scheduler| {
        scheduler.set_first_task(current_task);
        scheduler.setup_special_tasks(mapper, &mut frame_allocator);
    });

    TIME_KEEPER.with_mut_ref(|keeper| keeper.schedule_counter.time.reset());
    // main task starts here
    // # SAFETY: ps2_device_1_task calls schedule once per loop
    let ps2_task = Task::new(
        String::from("PS/2 Deivce 1 Task"),
        ps2_device_1_task,
        mapper,
        &mut frame_allocator,
    );

    // # SAFETY: process_keys calls schedule once per loop
    let keys_task = Task::new(
        String::from("Proccess keys"),
        process_keys,
        mapper,
        &mut frame_allocator,
    );

    // # SAFETY: kernal_shell calle schedule once per loop
    let shell_task = Task::new(
        String::from("Kernal Shell"),
        kernal_shell,
        mapper,
        &mut frame_allocator,
    );

    let task_1 = Task::new(String::from("Task 1"), task_1, mapper, &mut frame_allocator);

    let task_2 = Task::new(String::from("Task 2"), task_2, mapper, &mut frame_allocator);

    unsafe { mapper.clean_up(&mut frame_allocator) };
    PMM.with_mut_ref(|v| v.replace(frame_allocator));

    let (_ps2_task, _keys_task, _shell_task) = SCHEDULER.with_mut_ref(|scheduler| {
        let ps2_task = scheduler.spawn_task(ps2_task);
        let keys_task = scheduler.spawn_task(keys_task);
        let shell_task = scheduler.spawn_task(shell_task);
        let _ = scheduler.spawn_task(task_1);
        let _ = scheduler.spawn_task(task_2);

        (ps2_task, keys_task, shell_task)
    });

    {
        // let mut mapper = unsafe { memory::init(VirtAddr::new(P_OFFSET)) };
        //
        // let page = Page::containing_address(VIRT_ADDR);
        //
        // let mut guard = PMM.acquire();
        // let pmm = guard.as_mut().unwrap();
        //
        // let frame = pmm.allocate_frame().unwrap();
        //
        // let flags =
        //     PageTableFlags::WRITABLE | PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
        //
        // unsafe {
        //     mapper.map_to(page, frame, flags, pmm);
        // }
        //
        // drop(guard);
        //
        // let ptr: *mut u8 = ptr::with_exposed_provenance_mut(ADDR);
        //
        // unsafe { ptr.write(69) };

        loop {
            sleep(Seconds(1).into());

            // println!("{}", unsafe { ptr.read() });
            // let cr3 = Cr3::read().0.start_address().as_u64();
            //
            // println!("cr3: {:X}", cr3);

            // debug!("Main task is still running properly");
        }
    }
}

const ADDR: usize = 0x050;
const VIRT_ADDR: VirtAddr = VirtAddr::new(ADDR as u64);

fn task_2() -> ! {
    let mut mapper = unsafe { memory::init(VirtAddr::new(P_OFFSET)) };

    let page = Page::containing_address(VIRT_ADDR);

    let mut guard = PMM.acquire();
    let pmm = guard.as_mut().unwrap();

    let frame = pmm.allocate_frame().unwrap();
    let frame = pmm.allocate_frame().unwrap();
    let frame = pmm.allocate_frame().unwrap();
    println!("frame: {frame:?}");

    let flags = PageTableFlags::WRITABLE | PageTableFlags::PRESENT;

    mapper
        .level_4_table_mut()
        .iter_mut()
        .find(|x| x.is_unused())
        .unwrap();

    unsafe {
        mapper.map_to(page, frame, flags, pmm);
    }

    drop(guard);

    // let table_count = mapper
    //     .level_4_table()
    //     .iter()
    //     .filter(|x| !x.is_unused())
    //     .for_each(|entry| {
    //         println!("task 1 {:#?}", entry);
    //     });

    // println!("table: count {table_count}");

    let ptr: *mut u8 = ptr::with_exposed_provenance_mut(ADDR);

    let addr = mapper.translate_addr(VirtAddr::new(ADDR as u64));
    println!("physc addr: {addr:?}");

    unsafe { ptr.write(69) };
    loop {
        sleep(Seconds(1).into());

        println!("task: 1");
        println!("{}", unsafe { ptr.read_volatile() });
        let cr3 = Cr3::read().0.start_address().as_u64();

        println!("cr3: {:X}", cr3);
        print!("\n");
    }
}

fn task_1() -> ! {
    let mut mapper = unsafe { memory::init(VirtAddr::new(P_OFFSET)) };

    let page = Page::containing_address(VIRT_ADDR);

    let mut guard = PMM.acquire();
    let pmm = guard.as_mut().unwrap();

    let frame = pmm.allocate_frame().unwrap();
    println!("frame: {frame:?}");

    let flags =
        PageTableFlags::WRITABLE | PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;

    unsafe {
        mapper.map_to(page, frame, flags, pmm);
    }

    drop(guard);

    let addr = mapper.translate_addr(VIRT_ADDR);
    println!("physc addr: {addr:?}");

    // let table_count = mapper
    //     .level_4_table()
    //     .iter()
    //     .filter(|x| !x.is_unused())
    //     .for_each(|entry| {
    //         println!("task 2 {:#?}", entry);
    //     });

    let ptr: *mut u8 = ptr::with_exposed_provenance_mut(ADDR);

    unsafe { ptr.write(67) };
    loop {
        sleep(Seconds(1).into());

        println!("task: 2");
        println!("{}", unsafe { ptr.read_volatile() });
        let cr3 = Cr3::read().0.start_address().as_u64();

        println!("cr3: {:X}", cr3);
        print!("\n");
    }
}

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

    let exit_handle = qemu_exit::X86::new(0xf4, 3);

    exit_handle.exit_failure();
}
