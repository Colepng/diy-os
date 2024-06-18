use core::arch::asm;

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

#[no_mangle]
pub extern "C" fn usermode() {
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
}
