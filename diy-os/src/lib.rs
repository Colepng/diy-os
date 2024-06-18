#![no_std]
#![crate_type = "rlib"]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_harness_main"]
#![feature(abi_x86_interrupt)]
#![feature(const_mut_refs)]
#![feature(negative_impls)]
#![feature(ascii_char)]
#![feature(ascii_char_variants)]
#![feature(anonymous_lifetime_in_impl_trait)]
#![feature(const_refs_to_static)]
#![feature(strict_provenance)]
#![feature(exposed_provenance)]
#![feature(layout_for_ptr)]
#![feature(naked_functions)]
#![feature(asm_const)]
#![feature(str_from_raw_parts)]
#![warn(clippy::pedantic, clippy::nursery, clippy::perf, clippy::style)]
#![deny(
    clippy::suspicious,
    clippy::correctness,
    clippy::complexity,
    clippy::missing_const_for_fn,
    unsafe_op_in_unsafe_fn,
    fuzzy_provenance_casts
)]
#![allow(
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    clippy::explicit_deref_methods
)]

use alloc::alloc::alloc;
use bootloader_api::BootInfo;
use core::{
    arch::{asm, global_asm},
    panic::PanicInfo,
};

#[cfg(test)]
use bootloader_api::entry_point;
extern crate alloc;

pub mod allocator;
pub mod console;
pub mod framebuffer;
pub mod gdt;
pub mod interrupts;
pub mod memory;
pub mod pit;
pub mod serial;
pub mod spinlock;
pub mod syscalls;
pub mod timer;

// #[link(name="mylib")]
// extern {
//     pub fn into_usermode();
// }
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

static string: &'static str = "hello";
const ptr: *const u8 = string.as_ptr();

// global_asm!(include_str!("../../usermode.asm"));
#[no_mangle]
pub extern "C" fn usermode() {
    let mut temp: usize = 0;

    unsafe {
        asm!(
            "mov rax, 1",
            "mov rdx, 35",
            "mov rsi, 30",
            "int 0x80",
            "push rax",
            "mov rax, 0",
            "mov rsi, rsp",
            "mov rdx, 1",
            "int 0x80",
        );
    }

    // use alloc::string::ToString;
    // let a_str = a.to_string();
    // syscalls::print(&a_str, a_str.len());
    // }
    // unsafe { x86_64::instructions::interrupts::software_interrupt::<0x80>() };
    // unsafe {
    //     asm!(
    //     "mov rax, 0",
    //     "ret",
    //     options(noreturn),
    //     );
    // }

    // loop {
    // }
    // syscalls::print("Hello", 5);
    // let len = 5;
    // let str: u64 = 0x100000000000 + 100;

    // unsafe {
    //     asm!(
    //         "mov rax, 0x0
    //         mov rsi, (0x100000000000 + 100)
    //         mov rdx, 5
    //         int 0x80"
    //         , options(noreturn)
    //         // in(reg) str,
    //         // in(reg) len,
    //     );
    // }
    // unsafe {
    //     asm!(
    //         "mov rax, 0x1
    //         mov rsi, 10
    //         mov rdx, 10
    //         int 0x80"
    //         , options(noreturn)
    //         // in(reg) str,
    //         // in(reg) len,
    //     );
    // }
}

pub trait Testable {
    #[allow(clippy::unused_unit)]
    fn run(&self) -> ();
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
}

pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    exit_qemu(QemuExitCode::Success);
}

pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failed);
    hlt_loop();
}

#[cfg(test)]
entry_point!(test_main);

/// Entry point for `cargo test`
#[cfg(test)]
#[no_mangle]
fn test_main(boot_info: &'static mut BootInfo) -> ! {
    init(boot_info);
    test_harness_main();
    hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}

pub fn init(boot_info: &'static mut BootInfo) -> &'static mut BootInfo {
    println!("Entering init");

    if let Some(framebuffer) = boot_info.framebuffer.take() {
        framebuffer::init(framebuffer);
        println!("Framebuffer Initialized");
    }

    gdt::init();
    println!("GDT Initialized");

    interrupts::init_idt();
    println!("IDT Initialized");
    unsafe { interrupts::PICS.acquire().initialize() };
    println!("PICS Initialized");
    interrupts::unmask();
    println!("Interrupts Unmasked");
    x86_64::instructions::interrupts::enable();
    println!("Interrupts Enabled");
    timer::setup_system_timer();
    println!("System timer Initialized");

    boot_info
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
