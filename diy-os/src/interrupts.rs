use lazy_static::lazy_static;
use pic8259::ChainedPics;
use x86_64::{
    set_general_handler,
    structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode},
    VirtAddr,
};

use crate::{
    gdt, pit, println,
    ps2::controller::{Inital, InitalTrait, ReadyToReadTrait, WaitingToReadTrait},
    syscalls,
};

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: crate::spinlock::Spinlock<ChainedPics> =
    crate::spinlock::Spinlock::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        set_general_handler!(&mut idt, general_handler);
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        unsafe {
            idt.invalid_opcode
                .set_handler_fn(invalid_opcode_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
            idt.page_fault
                .set_handler_fn(page_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
            idt.general_protection_fault
                .set_handler_fn(general_protection_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
            idt[InterruptIndex::Timer.as_u8()].set_handler_fn(timer_interrupt_handler);
            idt[InterruptIndex::Keyboard.as_u8()].set_handler_fn(keyboard_interrupt_handler);
            idt[0x80]
                .set_handler_addr(VirtAddr::new(
                    (syscalls::system_call_handler_wrapper as usize)
                        .try_into()
                        .unwrap(),
                ))
                .set_privilege_level(x86_64::PrivilegeLevel::Ring3)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt
    };
}

pub fn init_idt() {
    IDT.load();
}

pub fn unmask() {
    unsafe { PICS.acquire().write_masks(0b1111_1000, 0b1111_1111) };
}

#[allow(clippy::needless_pass_by_value)]
fn general_handler(stack_frame: InterruptStackFrame, index: u8, error_code: Option<u64>) {
    panic!(
        "EXCEPTION: unknown fault\n{:#?}, \nerror code {:?}, \nindex {:x}",
        stack_frame, error_code, index
    );
}

extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
    panic!("invalid_opcode \n{:#?}", stack_frame,)
}

extern "x86-interrupt" fn general_protection_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "EXCEPTION: GENERAL PROTECTION \n{:#?}\nerror code {:b}",
        stack_frame, error_code
    );
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    panic!(
        "EXCEPTION: page fault\n{:#?}, \nerror code {:?}",
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
    {
        let mut con = crate::ps2::CONTROLLER.acquire();

        match con.take() {
            Some(controller) => {
                let (controller, result): (_, u8) =
                    WaitingToReadTrait::<u8>::try_read(controller.as_reader())
                        .unwrap()
                        .read();
                println!("{}", result);
                con.replace(controller);
            }
            None => {}
        }
    }

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
