use crate::log::info;
use crate::timer::{Duration, TIME_KEEPER};
use alloc::boxed::Box;
use alloc::collections::linked_list::LinkedList;
use alloc::string::String;
use spinlock::Spinlock;
use x86_64::VirtAddr;
use x86_64::structures::paging::{FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB};
use x86_64::{registers::control::Cr3, structures::paging::PhysFrame};

// TEMP, this is not checked big UB!
// Setup some way to track used pages
static STACK_COUNTER: Spinlock<u64> = Spinlock::new(0);
const START_ADDR: VirtAddr = VirtAddr::new(0x0000_0000_0804_aff8);

pub static SCHEDULER: Spinlock<Scheduler> = Spinlock::new(Scheduler::new());

pub struct Scheduler {
    current_task: Option<Box<Task>>,
    ready_tasks: LinkedList<Box<Task>>,
}

impl Scheduler {
    const fn new() -> Self {
        Self {
            current_task: Option::None,
            ready_tasks: LinkedList::new(),
        }
    }

    /// Sets the first task of this [`Scheduler`].
    ///
    /// # Panics
    ///
    /// Panics if the first task was already test.
    pub fn set_first_task(&mut self, task: Box<Task>) {
        assert!(self.current_task.is_none());

        self.current_task.replace(task);
    }

    pub fn get_current_task(&self) -> Option<&Task> {
        self.current_task.as_deref()
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct Task {
    pub stack: VirtAddr,
    pub stack_top: VirtAddr,
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

        info("allocated new stack");

        let new_task = unsafe { Self::crate_new_task(common_name, task_fn, stack_page) };

        SCHEDULER.with_mut_ref(|scheduler| {
            scheduler.ready_tasks.push_back(new_task);
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
        new_task: fn() -> !,
        stack_page: Page<Size4KiB>,
    ) -> Box<Self> {
        let end_of_stack_addr = stack_page.start_address() + 0x1000;
        let mut stack_ptr: *mut u64 = end_of_stack_addr.as_mut_ptr();

        let task = Self::allocate_task(
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
/// `current_task` must also be the current task
// arg1: rdi
// arg2: rsi
#[unsafe(naked)]
pub unsafe extern "sysv64" fn switch_to_task(current_task: *mut Task, next_task: *mut Task) {
    core::arch::naked_asm!(
        "mov [rdi+16], r8", // store rax in task struct
        "mov [rdi], rsp",   // store rsp in task struct
        "mov [rdi+8], rbp", // store rbp
        "mov rsp, [rsi]",   // load rsp from the next task
        "mov rbp, [rsi+8]", //load rbp
        // "mov [{a}+4], rbp",
        "mov r8, [rsi+16]", // load fax
        "ret",
    );
}

fn first_time_task_cleanup() {
    x86_64::instructions::interrupts::enable();
    SCHEDULER.release();
}

pub fn schedule() {
    x86_64::instructions::interrupts::disable();
    let elapsed = TIME_KEEPER.with_mut_ref(|keeper| {
        let elep = keeper.schedule_counter.time;
        keeper.schedule_counter.time.reset();
        elep
    });

    SCHEDULER.with_mut_ref(|scheduler| {
        if !scheduler.ready_tasks.is_empty() {
            let mut current_task = scheduler.current_task.take().unwrap();
            let mut next_task = scheduler.ready_tasks.pop_front().unwrap();
            let current_task_ptr = Box::<Task>::as_mut_ptr(&mut current_task);
            let next_task_ptr = Box::<Task>::as_mut_ptr(&mut next_task);

            current_task.time_used += elapsed;

            scheduler.current_task.replace(next_task);
            scheduler.ready_tasks.push_back(current_task);

            unsafe { switch_to_task(current_task_ptr, next_task_ptr) };
        }
    });
    x86_64::instructions::interrupts::enable();
}
