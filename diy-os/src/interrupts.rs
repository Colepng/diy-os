use core::{arch::asm, usize};

use lazy_static::lazy_static;
use pic8259::ChainedPics;
use x86_64::{structures::idt::{InterruptDescriptorTable, InterruptStackFrame}, VirtAddr};

use crate::{gdt, pit, println};

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: crate::spinlock::Spinlock<ChainedPics> =
    crate::spinlock::Spinlock::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.general_protection_fault
            .set_handler_fn(general_protection_handler);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
            idt[InterruptIndex::Timer.as_u8()].set_handler_fn(timer_interrupt_handler);
            idt[InterruptIndex::Keyboard.as_u8()].set_handler_fn(keyboard_interrupt_handler);
            idt[0x80].set_handler_addr(VirtAddr::new(system_call_handler_wrapper as u64));
        }
        idt
    };
}

pub fn init_idt() {
    IDT.load();
}

pub fn unmask() {
    unsafe { PICS.acquire().write_masks(0b1111_1100, 0b1111_1111) };
}

/// Which sys call is passed through rax
#[naked]
unsafe extern "sysv64" fn system_call_handler_wrapper() {
    unsafe { 
        asm!("mov rcx, rsp
              sub rsp, 8 // align stack pointer
              mov rdi, rax
              call {0}
              add rsp, 8 // reset stack pointer
              iretq
              ", sym system_call_handler, options(noreturn));
    }
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
    let message = unsafe {
        core::str::from_raw_parts(ptr, len)
    };
    // //
    println!("{message}");
}

extern "sysv64" fn add() {
}

extern "x86-interrupt" fn general_protection_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "EXCEPTION: GENERAL\n{:#?}\nerror code {:b}",
        stack_frame, error_code
    );
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    panic!(
        "EXCEPTION: DOUBLE FAULT\n{:#?}, \nerror code {}",
        stack_frame, error_code
    );
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    let mut counter = pit::SLEEP_COUNTER.acquire();
    *counter = (*counter).saturating_sub(1);

    unsafe {
        PICS.acquire()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    unsafe {
        PICS.acquire()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIndex {
    const fn as_u8(self) -> u8 {
        self as u8
    }
}

#[test_case]
fn test_breakpoint_exception() {
    // invoke a breakpoint exception
    x86_64::instructions::interrupts::int3();
}
