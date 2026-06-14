use core::arch::naked_asm;

use thiserror::Error;
use x86_64::structures::paging::{FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB};

use crate::{P_OFFSET, elf, filesystem::FileTrait};

/// Jumps to `entry` in ring 3, setting the stack, with interrupts enabled
///
/// # Safety
/// The page where `entry` is mapped readable, user accessible and present.
/// The data on the page must also be valid code.
///
/// `stack_addr` is mapped, read/writeable, user accessible and present.
///
/// Both addressing must be in the lower half.
// Entry is passed in rd
// stack rsi
#[unsafe(naked)]
pub unsafe extern "sysv64" fn into_usermode(entry: u64, stack_addr: u64) -> ! {
    naked_asm!(
        "cli",
        "push {user_ss}",
        "push rsi",
        "push 0x202",
        "push {user_cs}",
        "push rdi",
        "iretq",
        user_ss = const 0x1B,
        user_cs = const 0x23,
    );
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

/// Loads an ELF executable into memory and returns its entry point.
///
/// # Errors
///
/// - [`ElfLoadingErorr::InvalidElfFile`] if the file doesn't have a valid ELF
///   signature.
/// - [`ElfLoadingErorr::FailedToAllocatePhysicalFrame`] if a physical frame
///   couldn't be allocated for the segment.
/// - [`ElfLoadingErorr::FailedToMap`] if the segment couldn't be mapped into
///   the target address space.
pub fn load_elf(
    file: &dyn FileTrait,
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<u64, ElfLoadingErorr> {
    let mut buffer = [0u8; 10000];

    let _ = file.read(&mut buffer).unwrap();
    let file_ptr = buffer.as_ptr();
    // .get_raw_bytes()
    // .ok_or(ElfLoadingErorr::EmptyFile)?
    // .as_ptr();

    assert!(file_ptr.is_aligned_to(align_of::<elf::Header>()));

    // SAFETY: Safe to assume file_ptr is not null and aligned since it comes from a reference.
    // Initialization checked immediately after.
    #[allow(clippy::cast_ptr_alignment)]
    let elf_header = unsafe { file_ptr.cast::<elf::Header>().as_ref_unchecked() };

    if !elf_header.is_valid_elf() {
        return Err(ElfLoadingErorr::InvalidElfFile);
    }

    let ptr = unsafe { file_ptr.byte_add(elf_header.program_header_table_offset) };

    assert!(ptr.is_aligned_to(align_of::<elf::ProgramHeaderTableEntry>()));

    #[allow(clippy::cast_ptr_alignment)]
    let program_header = unsafe { &*ptr.cast::<elf::ProgramHeaderTableEntry>() };

    let ph_virtaddr = program_header.virtual_address;

    let page_for_load: Page<Size4KiB> = Page::containing_address(ph_virtaddr);

    // Has to be writable to write the load segment to the page in the first place
    let flags =
        PageTableFlags::USER_ACCESSIBLE | PageTableFlags::WRITABLE | PageTableFlags::PRESENT;
    // | PageTableFlags::NO_CACHE; //| PageTableFlags::
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
            .add(usize::try_from(program_header.offset).unwrap())
            // .add(elf_header.program_header_table_offset)
            // .add(elf_header.program_header_entry_size as usize)
            .copy_to_nonoverlapping(
                core::ptr::with_exposed_provenance_mut(
                    usize::try_from(P_OFFSET).unwrap()
                        + usize::try_from(frame.start_address().as_u64()).unwrap(),
                ),
                usize::try_from(program_header.size_of_segment_file).unwrap(),
            );
    }

    Ok(elf_header.entery_address.as_u64())
}
