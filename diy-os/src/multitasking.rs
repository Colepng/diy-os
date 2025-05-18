use crate::log::{self, debug};
use crate::spinlock::Spinlock;
use crate::timer::{Duration, TIME_KEEPER};
use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use core::ptr;
use x86_64::VirtAddr;
use x86_64::structures::paging::{FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB};
use x86_64::{registers::control::Cr3, structures::paging::PhysFrame};

// TEMP, this is not checked big UB!
// Setup some way to track used pages
static STACK_COUNTER: Spinlock<u64> = Spinlock::new(0);
const START_ADDR: VirtAddr = VirtAddr::new(0x0000_0000_0804_aff8);

pub static CURRENT_TASK: Spinlock<Option<&mut Task>> = Spinlock::new(Option::None);

#[derive(Debug)]
#[repr(C)]
pub struct Task {
    pub stack: VirtAddr,
    pub stack_top: VirtAddr,
    pub next: *const Task,
    rax: u64,
    pub cr3: PhysFrame<Size4KiB>,
    pub time_used: Duration,
    pub common_name: String,
}

unsafe impl Send for Task {}

#[derive(thiserror::Error, Debug)]
pub enum TaskBuildError {
    #[error("The first task was not crated before trying to crate new tasks")]
    MissingFirstTaskBeforeAllocatingNewTask,
}

impl Task {
    #[allow(clippy::new_ret_no_self)]
    /// Allocates and insets a new task the linked list
    ///
    /// # Safety
    /// callers must ensure the function ptr calls the [`schedule`] frequently.
    ///
    /// # Errors
    /// Will error if the first task is not setup manually
    pub unsafe fn new(
        common_name: String,
        task_fn: fn() -> !,
        mapper: &mut impl Mapper<Size4KiB>,
        frame_alloc: &mut impl FrameAllocator<Size4KiB>,
    ) -> Result<(), TaskBuildError> {
        let num_of_stacks = STACK_COUNTER.with_mut_ref(|counter| {
            let temp = *counter;
            *counter += 1;
            temp
        });

        let addr = START_ADDR + 0x1000 * num_of_stacks;

        let stack_page = unsafe { allocate_stack(addr, mapper, frame_alloc) };

        log::info("allocated new stack");

        CURRENT_TASK.with_mut_ref(|task| {
            let current_task = task.as_mut();

            let current_task = current_task
                .ok_or(TaskBuildError::MissingFirstTaskBeforeAllocatingNewTask)
                .unwrap();

            let new_task =
                unsafe { Self::crate_new_task(common_name, current_task, task_fn, stack_page) };

            Box::leak(new_task);
        });

        Ok(())
    }

    pub fn allocate_task(
        common_name: String,
        rax: u64,
        top_of_stack: VirtAddr,
        stack: VirtAddr,
    ) -> Box<Self> {
        let task = Self {
            stack,
            stack_top: top_of_stack,
            next: core::ptr::null(),
            rax,
            cr3: Cr3::read().0,
            time_used: Duration::new(),
            common_name,
        };

        Box::new(task)
    }

    /// Allocates and insets a new task the linked list
    ///
    /// # Safety
    /// Callers must insure that the stack page is unused and that the function ptr
    /// calls the [`schedule`] frequently.
    unsafe fn crate_new_task(
        common_name: String,
        current_task: &mut Self,
        new_task: fn() -> !,
        stack_page: Page<Size4KiB>,
    ) -> Box<Self> {
        let end_of_stack_addr = stack_page.start_address() + 0x1000;
        let mut stack_ptr: *mut u64 = end_of_stack_addr.as_mut_ptr();

        let mut task = Self::allocate_task(
            common_name,
            0,
            end_of_stack_addr,
            end_of_stack_addr - (8 * 2),
        );

        // Setup the new stack
        unsafe {
            stack_ptr = stack_ptr.offset(-1);
            *stack_ptr = new_task as u64;
            stack_ptr = stack_ptr.offset(-1);
            *stack_ptr = first_time_task_cleanup as u64;
        }

        let old_next = current_task.next;

        current_task.next = ptr::from_ref(task.as_ref());
        task.next = old_next;

        task
    }
}

/// Allocates and Maps page frame for stack
///
/// Returns the page for the stack.
/// # Safety
///
/// Assumes this address is being kept tracked off and is unused.
pub unsafe fn allocate_stack(
    addr: VirtAddr,
    mapper: &mut impl Mapper<Size4KiB>,
    frame_alloc: &mut impl FrameAllocator<Size4KiB>,
) -> Page<Size4KiB> {
    let page_stack: Page<Size4KiB> = Page::containing_address(addr);
    let stack_flags = PageTableFlags::USER_ACCESSIBLE
        | PageTableFlags::WRITABLE
        | PageTableFlags::PRESENT
        | PageTableFlags::NO_CACHE;

    let stack_frame = frame_alloc.allocate_frame().unwrap();

    // map page for stack
    unsafe {
        mapper
            .map_to_with_table_flags(
                page_stack,
                stack_frame,
                stack_flags,
                stack_flags,
                frame_alloc,
            )
            .unwrap()
            .flush();
    }

    page_stack
}

/// Switch to the next task in the linked list
///
/// # Safety
/// Callers must insure that interrupts are disabled while this function is being called.
/// Also that both ptr are non null.
// arg1: rdi
#[unsafe(naked)]
pub unsafe extern "sysv64" fn switch_to_task(current_task: *mut *mut Task) {
    core::arch::naked_asm!(
        "mov rsi, [rdi]",    //load the task ptr to rsi
        "mov [rsi+24], r8",  // store rax in task struct
        "mov [rsi], rsp",    // store rsp in task struct
        "mov [rsi+8], rbp",  // store rbp
        "mov rdx, [rsi+16]", // save the new task ptr
        "mov [rdi], rdx",
        "mov rsi, rdx",     // move the next task to rsi
        "mov rsp, [rsi]",   // load rsp from the next task
        "mov rbp, [rsi+8]", //load rbp
        // "mov [{a}+4], rbp",
        "mov r8, [rsi+24]", // load fax
        "ret",
    );
}

fn first_time_task_cleanup() {
    CURRENT_TASK.release();
    x86_64::instructions::interrupts::enable();
}

pub fn schedule() {
    x86_64::instructions::interrupts::disable();
    let elapsed = TIME_KEEPER.with_mut_ref(|keeper| {
        let elep = keeper.schedule_counter.time;
        keeper.schedule_counter.time.reset();
        elep
    });

    CURRENT_TASK.with_mut_ref(|current_task| {
        let task = current_task.as_mut().unwrap();
        task.time_used += elapsed;
        let ptr = (task as *mut &mut Task) as *mut *mut Task;
        unsafe { switch_to_task(ptr) };
    });
    x86_64::instructions::interrupts::enable();
}
