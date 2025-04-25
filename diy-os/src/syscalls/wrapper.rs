#![allow(dead_code)]
use core::arch::asm;
use core::arch::naked_asm;

/// Parameters to functions are passed in the registers rdi, rsi, rdx, rcx, r8, r9, and further
/// values are passed on the stack in reverse order
///
/// rax return
///
#[unsafe(naked)]
unsafe extern "sysv64" fn sys_call<Arg1, Arg2, Arg3, Arg4, Arg5>(
    call: u64,
    arg1: Arg1,
    arg2: Arg2,
    arg3: Arg3,
    arg4: Arg4,
    arg5: Arg5,
) {
    naked_asm!("push rcx",
        "mov rax, rdi",
        "call {func}",
        "pop rcx",
        "ret",
        func = sym x86_64::instructions::interrupts::software_interrupt::<0x080>,
    );
}

pub fn print(str: &str) {
    let len = str.len();
    let ptr = str.as_ptr();

    unsafe { sys_call::<usize, usize, (), (), ()>(0, ptr as usize, len, (), (), ()) }
}

pub fn add(num: usize, other: usize) -> usize {
    unsafe { sys_call::<usize, usize, (), (), ()>(1, num, other, (), (), ()) }

    let ret: usize;

    unsafe {
        asm!("mov rax, {0}", out(reg) ret);
    }

    ret
}
