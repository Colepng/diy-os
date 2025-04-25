mod wrapper;

use crate::println;
use core::arch::asm;
use core::arch::naked_asm;

/// Which sys call is passed through rax
/// # Safety
/// Syscall id must be passed through rax
/// Rax contains return value
#[unsafe(naked)]
pub unsafe extern "sysv64" fn system_call_handler_wrapper() {
    // loop {}
    naked_asm!("mov rcx, rsp
        sub rsp, 8 // align stack pointer
        mov rdi, rax
        call {0}
        add rsp, 8 // reset stack pointer
        iretq
        ", sym system_call_handler);
}

extern "sysv64" fn system_call_handler(syscall_index: usize) {
    // println!("calling syscall {syscall_index}");

    unsafe {
        SYS_CALLS[syscall_index]();
    };
}

// If the class is INTEGER, the next available register of the sequence %rdi,
// %rsi, %rdx, %rcx, %r8 and %r9 is used

const SYS_CALLS: [unsafe extern "sysv64" fn(); 2] = [print, add];

// pointer is arg 1, len is arg 2
extern "sysv64" fn print() {
    let mut ptr: *const u8;
    let mut len: usize;

    unsafe {
        asm!(
            "mov {ptr}, rsi",
            "mov {len}, rdx",
            ptr = out(reg) ptr,
            len = out(reg) len,
        );
    }

    // println!("ptr: {}", ptr);
    // println!("len: {}", len);

    // println!("got {:?}, {:?}", ptr, len);

    // println!("in print");
    let message = unsafe { core::str::from_raw_parts(ptr, len) };
    // //
    println!("{message}");
}

extern "sysv64" fn add() {
    let mut num1: usize;
    let mut num2: usize;

    unsafe {
        asm!(
            "mov {num1}, rsi",
            "mov {num2}, rdx",
            num1 = out(reg) num1,
            num2 = out(reg) num2,
        );
    }

    let ret = num1 + num2;

    unsafe {
        asm!(
            "mov rax, {ret}",
            ret = in(reg) ret,
        );
    }
}
