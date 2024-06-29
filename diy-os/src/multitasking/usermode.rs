use super::ExitCode;
use core::arch::asm;

use thiserror::Error;
use x86_64::structures::paging::{FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB};

use crate::elf;

pub extern "sysv64" fn into_usermode(entry: u64, stack_addr: u64) {
    unsafe {
        asm!(
        "cli",
        // rdi = user args
        // rsi = entry point for userspace
        // rdx = user space stack
        // "mov rax, 0x18 | 3",
        // "mov ax, ( 4 * 8 ) | 3",
        // "mov ds, ax",
        // "mov es, ax",
        // "mov fs, ax",
        // "mov gs, ax",

        // "push rax", // user data
        // "push rsp", // user stack
        // "pushf", // rflags = inerrupts + reservied bit
        // "push 0x23", // selctor 0x20 + rpl 3
        // "push {}", // entry point
        // fake iret frame
         "mov ax, (4 * 8) | 3",
         "mov ds, ax",
         "mov es, ax",
         "mov fs, ax",
         "mov gs, ax",
         // //stackfame
         "mov rax, {1}",
         "push (4 * 8) | 3 ",
         "push rax",
         "push 0x202",
         "push ( 3 * 8 ) | 3",
         "push {0}",
         "iretq",
         in(reg) entry,
         in(reg) stack_addr,
         options(noreturn),
        )
    }
}

#[derive(Error, Debug)]
pub enum ElfLoadingErorr {
    #[error("Failed to load because file was empty")]
    EmptyFile,
    #[error("Falid to load because file had a invalid signature")]
    InvalidElfFile,
    #[error("Failed to allocate physical frame")]
    FailedToAllocatePhysicalFrame,
    #[error("Failed to map page, caused by {0:?}")]
    FailedToMap(x86_64::structures::paging::mapper::MapToError<Size4KiB>),
}

pub fn load_elf_and_jump_into_it(
    file: &crate::filesystem::ustar::File,
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<ExitCode, ElfLoadingErorr> {
    let file_ptr = file
        .get_raw_bytes()
        .ok_or(ElfLoadingErorr::EmptyFile)?
        .as_ptr();

    // SAFETY: Safe to assume file_ptr is not null and aligned since it comes from a reference.
    // Initialization checked immediately after.
    let elf_header = unsafe { file_ptr.cast::<elf::Header>().as_ref_unchecked() };

    if !elf_header.is_valid_elf() {
        return Err(ElfLoadingErorr::InvalidElfFile);
    }

    let program_header = unsafe {
        &*file_ptr
            .byte_offset(elf_header.program_header_table_offset as isize)
            .cast::<elf::ProgramHeaderTableEntry>()
    };

    let ph_virtaddr = program_header.virtual_address;

    let page_for_load: Page<Size4KiB> = Page::containing_address(ph_virtaddr);

    // Has to be writable to write the load segment to the page in the first place
    let flags = PageTableFlags::USER_ACCESSIBLE
        | PageTableFlags::WRITABLE
        | PageTableFlags::PRESENT
        | PageTableFlags::NO_CACHE; //| PageTableFlags::
                                    //
    let frame = frame_allocator
        .allocate_frame()
        .ok_or(ElfLoadingErorr::FailedToAllocatePhysicalFrame)?;

    // Setup page for load
    unsafe {
        mapper
            .map_to_with_table_flags(page_for_load, frame, flags, flags, frame_allocator)
            .map_err(ElfLoadingErorr::FailedToMap)?
            .flush();
    }

    unsafe {
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

    let stack_frame = frame_allocator
        .allocate_frame()
        .ok_or(ElfLoadingErorr::FailedToAllocatePhysicalFrame)?;

    // map page for stack
    unsafe {
        mapper
            .map_to_with_table_flags(page_stack, stack_frame, stack_flags, stack_flags, frame_allocator)
            .map_err(ElfLoadingErorr::FailedToMap)?
            .flush();
    }

    into_usermode(
        program_header.virtual_address.as_u64(),
        page_stack.start_address().as_u64() + 0x1000,
    );

    Ok(ExitCode::Successful)
}

pub fn load_elf(
    file: &crate::filesystem::ustar::File,
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(u64, u64), ElfLoadingErorr> {
    let file_ptr = file
        .get_raw_bytes()
        .ok_or(ElfLoadingErorr::EmptyFile)?
        .as_ptr();

    // SAFETY: Safe to assume file_ptr is not null and aligned since it comes from a reference.
    // Initialization checked immediately after.
    let elf_header = unsafe { file_ptr.cast::<elf::Header>().as_ref_unchecked() };

    if !elf_header.is_valid_elf() {
        return Err(ElfLoadingErorr::InvalidElfFile);
    }

    let program_header = unsafe {
        &*file_ptr
            .byte_offset(elf_header.program_header_table_offset as isize)
            .cast::<elf::ProgramHeaderTableEntry>()
    };

    let ph_virtaddr = program_header.virtual_address;

    let page_for_load: Page<Size4KiB> = Page::containing_address(ph_virtaddr);

    // Has to be writable to write the load segment to the page in the first place
    let flags = PageTableFlags::USER_ACCESSIBLE
        | PageTableFlags::WRITABLE
        | PageTableFlags::PRESENT
        | PageTableFlags::NO_CACHE; //| PageTableFlags::
                                    //
    let frame = frame_allocator
        .allocate_frame()
        .ok_or(ElfLoadingErorr::FailedToAllocatePhysicalFrame)?;

    // Setup page for load
    unsafe {
        mapper
            .map_to_with_table_flags(page_for_load, frame, flags, flags, frame_allocator)
            .map_err(ElfLoadingErorr::FailedToMap)?
            .flush();
    }

    unsafe {
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

    let stack_frame = frame_allocator
        .allocate_frame()
        .ok_or(ElfLoadingErorr::FailedToAllocatePhysicalFrame)?;

    // map page for stack
    unsafe {
        mapper
            .map_to_with_table_flags(page_stack, stack_frame, stack_flags, stack_flags, frame_allocator)
            .map_err(ElfLoadingErorr::FailedToMap)?
            .flush();
    }

    Ok((program_header.virtual_address.as_u64(), page_stack.start_address().as_u64() + 0x1000))
}

// #[no_mangle]
// pub extern "C" fn usermode() {
//     unsafe {
//         asm!(
//             "mov rax, 1",
//             "mov rdx, 35",
//             "mov rsi, 30",
//             "int 0x80",
//             "push rax",
//             "mov rax, 0",
//             "mov rsi, rsp",
//             "mov rdx, 1",
//             "int 0x80",
//         );
//     }
// }
