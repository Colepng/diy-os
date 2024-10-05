use core::arch::asm;

use super::Process;

/// Entry is passed in rdi
/// stack rsi
#[naked]
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
            "mov rax, rsi",
            "push (4 * 8) | 3 ",
            "push rax",
            "push 0x202",
            "push ( 3 * 8 ) | 3",
            "push rdi",
            "iretq",
            options(noreturn),
        )
    }
}
/// Caller must disable and enable interrupts before and after call
/// Functions preserve the registers:
///     rbx, rsp, rbp, r12, r13, r14, and r15;
/// Scratch registers:
///     rax, rdi, rsi, rdx, rcx, r8, r9, r10, r11
#[naked]
pub unsafe extern "sysv64" fn switch_to_task<'a>(
    previous_task: &'a mut Process,
    next_task: &'a mut Process,
) {
    // rdi holds current process
    // rsi hold next process
    unsafe {
        asm!(
            // Save previous task state
            "push rbx", 
            "push rbp", 
            "push r12", 
            "push r13", 
            "push r14", 
            "push r15",

            "mov [rdi], rsp", // store rsp in previous task

            // Load next task state
            "mov rsp, [rdx]", // load new stack
                              
            "pop r15",
            "pop r14",
            "pop r13",
            "pop r12",
            "pop rbp",
            "pop rbx",


            // load next task state
            // load rsp of next task
            // stack is now new task stack
            // store cr3(page directory) of the next task
            // change TSS rsp0 field to the top of stack address
            // check if the virtual address space is the same(if the level 4 page table is the same I think)
            // if not reload save registers This is the file that actually changes between tasks. It defines a function, switchTask(), which does all the magic. It saves all registers to from and loads them from to. It is trickier than you might think. Its function prototype is 
            // load next task eip from the new kernel stack

            options(noreturn),
        );
    }
}
