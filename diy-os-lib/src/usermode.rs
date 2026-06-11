use core::arch::naked_asm;

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
pub unsafe extern "sysv64" fn into_usermode(entry: u64, stack_addr: u64) {
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
