mod wrapper;

use x86_64::VirtAddr;
use x86_64::registers::control::Efer;
use x86_64::registers::control::EferFlags;
use x86_64::registers::model_specific::KernelGsBase;
use x86_64::registers::model_specific::LStar;
use x86_64::registers::model_specific::SFMask;
use x86_64::registers::model_specific::Star;
use x86_64::registers::rflags::RFlags;

use crate::gdt::GDT;
use core::arch::naked_asm;

static mut KERNEL_RSP: (u64, u64) = (0, 0);

pub fn init_syscalls() {
    unsafe {
        Efer::update(|flags| {
            flags.insert(EferFlags::SYSTEM_CALL_EXTENSIONS);
        });
    }

    Star::write(
        GDT.1.user_code_selector,
        GDT.1.user_data_selector,
        GDT.1.kernel_code_selector,
        GDT.1.kernel_data_selector,
    )
    .unwrap();

    LStar::write(VirtAddr::new(syscall_entry as *const () as u64));

    SFMask::write(RFlags::INTERRUPT_FLAG);

    KernelGsBase::write(VirtAddr::new(&raw const KERNEL_RSP as u64));

    // TODO change to a different stack
    unsafe {
        KERNEL_RSP.1 = rsp();
    }
}

#[allow(clippy::inline_always)]
#[inline(always)]
fn rsp() -> u64 {
    let rsp: u64;

    unsafe { core::arch::asm!("mov {stack}, rsp", stack = out(reg) rsp) }

    rsp
}

/// Entry point for the `syscall` instruction.
///
/// # Safety
/// Unsafe to call anywhere but as a syscall entry stub
///
/// On entry from userspace:
///   - rcx = user RIP (saved by `syscall`)
///   - r11 = user RFLAGS (saved by `syscall`)
///   - rax = syscall number
///   - rdi, rsi, rdx, r10, r8, r9 = syscall args (note: r10, NOT rcx)
///   - rsp = user stack (NOT yet switched to kernel)
///   - interrupts are disabled (because `SFMask` cleared IF)
#[unsafe(naked)]
unsafe extern "sysv64" fn syscall_entry() {
    naked_asm!(
        // Switch to kernel GS, so gs:[0] / gs:[8] reach CPU_LOCAL.
        "swapgs",

        // Stash user rsp, load kernel rsp.
        "mov gs:[0], rsp",
        "mov rsp, gs:[0]",

        // Save user return state (so we can sysretq back).
        "push rcx",             // user RIP
        "push r11",             // user RFLAGS

        // Save callee-saved registers we might clobber in Rust.
        "push rbx",
        "push rbp",
        "push r12",
        "push r13",
        "push r14",
        "push r15",

        // Set up SysV args for dispatcher:
        //   dispatcher(syscall_id, arg0, arg1, arg2, arg3, arg4)
        //              rdi         rsi   rdx   rcx   r8    r9
        //
        // Current values:
        //   rax = syscall id
        //   rdi = arg0, rsi = arg1, rdx = arg2, r10 = arg3, r8 = arg4, r9 = arg5
        //
        // Shift everything so syscall id ends up in rdi.
        "mov r9, r8",           // arg4 → r9 slot (was arg5; we drop arg5)
        "mov r8, r10",          // arg3 → r8 slot
        "mov rcx, rdx",         // arg2 → rcx slot
        "mov rdx, rsi",         // arg1 → rdx slot
        "mov rsi, rdi",         // arg0 → rsi slot
        "mov rdi, rax",         // syscall id → rdi slot

        "call {dispatch}",
        // Return value is in rax — leave it alone.

        // Restore saved registers.
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbp",
        "pop rbx",

        "pop r11",              // user RFLAGS
        "pop rcx",               // user RIP

        // Switch back to user stack.
        "mov rsp, gs:[0]",
        "swapgs",

        "sysretq",

        dispatch = sym syscall_dispatch,
    );
}

#[unsafe(no_mangle)]
pub extern "sysv64" fn syscall_dispatch(
    syscall_id: usize,
    arg0: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
) -> usize {
    crate::println!("syscall {syscall_id} args {arg0:#x} {arg1:#x} {arg2:#x} {arg3:#x} {arg4:#x}");
    0
}
