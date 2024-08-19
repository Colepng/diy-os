#![no_std]
#![no_main]
#![feature(optimize_attribute)]
#![feature(custom_test_frameworks)]
#![feature(naked_functions)]
#![feature(never_type)]
#![feature(pointer_is_aligned_to)]
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
use diy_os::ps2::controllers::{
    gernaric::Generic, ConfigurationByte, EnableSecondPort, EnabledOrDisabled, PS2Controller,
    ReadConfigurationByte, WriteConfigurationByte,
};
use diy_os::{elf, filesystem::ustar, hlt_loop, init, println};
use x86_64::structures::paging::{FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB};

static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    let mut mappings = Mappings::new_default();
    mappings.physical_memory = Some(Mapping::Dynamic);
    config.mappings = mappings;

    config
};

entry_point!(main_wrapper, config = &BOOTLOADER_CONFIG);

#[no_mangle]
extern "Rust" fn main_wrapper(boot_info: &'static mut BootInfo) -> ! {
    match main(boot_info) {
        Err(err) => panic!("{err:?}"),
    }
}

#[no_mangle]
extern "Rust" fn main(boot_info: &'static mut BootInfo) -> anyhow::Result<!> {
    let (boot_info, _frame_allocator, _mapper) = init(boot_info, 100)?;

    let ramdisk_addr = boot_info.ramdisk_addr.into_option().unwrap();
    let ramdisk = unsafe { ustar::Ustar::new(ramdisk_addr.try_into()?) };

    println!("Hello, world!");

    let elf_file = &ramdisk.get_files()[0];

    let mut gernaric = Generic::new();

    gernaric.initialize();

    {
        diy_os::ps2::CONTROLLER.acquire().replace(gernaric);
    }

    // let _ = load_elf_and_jump_into_it(elf_file, &mut mapper, &mut frame_allocator);

    hlt_loop();
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
