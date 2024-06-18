#![no_std]
#![no_main]
#![feature(optimize_attribute)]
#![feature(custom_test_frameworks)]
#![feature(naked_functions)]
#![feature(asm_const)]
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
use core::{
    alloc::GlobalAlloc,
    arch::{asm, global_asm},
    fmt::write,
    mem::transmute,
    ops::Add,
    panic::PanicInfo,
    u64,
};
use diy_os::{
    allocator, hlt_loop, init,
    memory::{self, BootInfoFrameAllocator},
    println,
};
use x86_64::{
    structures::paging::{FrameAllocator, Mapper, Page, PageTableFlags},
    VirtAddr,
};

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

    // println!("going to sleep");
    //
    // timer::sleep(1000);
    //
    // println!("wakign up");
    //
    // print("print sys call");

    let frame = frame_allocator.allocate_frame().unwrap();
    let page = Page::containing_address(VirtAddr::new(0x90000));
    let flags = PageTableFlags::USER_ACCESSIBLE
        | PageTableFlags::WRITABLE
        | PageTableFlags::PRESENT
        | PageTableFlags::NO_CACHE;
    unsafe {
        mapper
            .map_to(page, frame, flags, &mut frame_allocator)
            .unwrap()
            .flush()
    };
    let addr: *mut u8 = page.start_address().as_mut_ptr();
    unsafe { core::ptr::copy_nonoverlapping(diy_os::usermode as *const u8, addr, 100) };

    // let byte1 = unsafe { addr.read() };
    // let byte2 = unsafe { addr.add(1).read() };
    // let byte3 = unsafe { addr.add(2).read() };
    // let byte4 = unsafe { addr.add(3).read() };
    //
    // let str: *const u8 = [072, 101, 108, 108, 111].as_ptr();
    // unsafe {
    //     core::ptr::copy_nonoverlapping(str, addr.add(100), 5);
    // };

    // println!("{:x}, {:x}, {:x}, {:x}", byte1, byte2, byte3, byte4);

    let stack_addr = unsafe { addr.add(0x1000) } as u64;

    println!("{}", addr.is_aligned_to(8));

    println!("entering userspace");

    diy_os::into_usermode(addr as u64, stack_addr);

    hlt_loop();
}

// const a_ptr: *const u64 = usermode as *const u64;
// const a: u64 = unsafe { transmute(a_ptr) };

// #[naked]
// extern "sysv64" fn jump_usermode() {
//     unsafe {
//         asm!("
//             extern test_user_function
//             mov ax, (4 * 8) | 3
//               mov ds, ax
//               mov es, ax
//               mov fs, ax
//               mov gs, ax
//
//               // stack frame setup
//               mov eax, esp
//               push (4 * 8) | 3 // data selector
//               push rax // current esp
//               pushf // eflags
//               push (3 * 8) | 3 // code selector (ring 3 code with bottom 2 bits set for ring 3)
// 	          push test_user_function // instruction address to return to
//               iret
//             ",
//             options(noreturn),
//             options()
//         );
//     }
// }

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
