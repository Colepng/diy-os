use crate::hlt_loop;
use crate::timer::{Duration, Miliseconds, TIME_KEEPER, TimeKeeper};
use alloc::collections::linked_list::LinkedList;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use log::{debug, info};
use spinlock::Spinlock;
use x86_64::VirtAddr;
use x86_64::instructions::interrupts::without_interrupts;
use x86_64::structures::paging::{FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB};
use x86_64::{registers::control::Cr3, structures::paging::PhysFrame};

pub mod mutex;

// TEMP, this is not checked big UB!
// Setup some way to track used pages
static STACK_COUNTER: Spinlock<u64> = Spinlock::new(0);
const START_ADDR: VirtAddr = VirtAddr::new(0x0000_0000_0804_aff8);

pub static SCHEDULER: Spinlock<Scheduler> = Spinlock::new(Scheduler::new());

pub struct Scheduler {
    current_task: Option<Arc<Spinlock<Task>>>,
    ready_tasks: LinkedList<Arc<Spinlock<Task>>>,
    blocked_tasks: LinkedList<Arc<Spinlock<Task>>>,
    dead_tasks: LinkedList<Arc<Spinlock<Task>>>,
    cleaner_task: Option<Arc<Spinlock<Task>>>,
    pub time_slice: Duration,
}

impl Scheduler {
    pub const TIME_SLICE_AMOUNT: Duration = Miliseconds(10).into();

    const fn new() -> Self {
        Self {
            current_task: None,
            ready_tasks: LinkedList::new(),
            blocked_tasks: LinkedList::new(),
            dead_tasks: LinkedList::new(),
            cleaner_task: None,
            time_slice: Self::TIME_SLICE_AMOUNT,
        }
    }

    /// Sets the first task of this [`Scheduler`].
    ///
    /// # Panics
    ///
    /// Panics if the first task was already set.
    pub fn set_first_task(&mut self, task: Task) {
        assert!(self.current_task.is_none());

        self.current_task.replace(Arc::new(Spinlock::new(task)));
    }

    pub fn setup_special_tasks(
        &mut self,
        mapper: &mut impl Mapper<Size4KiB>,
        frame_alloc: &mut impl FrameAllocator<Size4KiB>,
    ) {
        let cleaner_task = Task::new(String::from("Cleaner"), cleaner_task, mapper, frame_alloc);
        let idle_task = Task::new(String::from("idle"), idle_task, mapper, frame_alloc);

        self.spawn_task(cleaner_task);
        self.spawn_task(idle_task);
    }

    pub fn get_current_task(&self) -> Option<Arc<Spinlock<Task>>> {
        self.current_task.clone()
    }

    pub fn spawn_task(&mut self, task: Task) -> Weak<Spinlock<Task>> {
        let task = Arc::new(Spinlock::new(task));

        let weak = Arc::downgrade(&task);

        self.ready_tasks.push_back(task);

        weak
    }

    /// Wakes up the sleeping tasks that have completed.
    pub fn wake_up_sleeping_tasks(&mut self, time_keeper: &mut TimeKeeper) {
        let instant = time_keeper.time_since_boot.time;

        let tasks: Vec<Arc<Spinlock<Task>>> = self
            .blocked_tasks
            .extract_if(|task| {
                task.with_ref(|task| {
                    if let State::Blocked(BlockedReason::SleepingUntil(until)) = task.state {
                        until <= instant
                    } else {
                        false
                    }
                })
            })
            .collect();

        for task in tasks {
            self.ready_task(task);
        }
    }

    fn ready_task(&mut self, task: Arc<Spinlock<Task>>) {
        without_interrupts(|| {
            task.with_mut_ref(|task| {
                task.state = State::ReadyToRun;
            });

            self.ready_tasks.push_back(task);
        });
    }

    pub const fn get_ready_tasks(&self) -> &LinkedList<Arc<Spinlock<Task>>> {
        &self.ready_tasks
    }

    pub const fn get_blocked_tasks(&self) -> &LinkedList<Arc<Spinlock<Task>>> {
        &self.blocked_tasks
    }

    fn print_tasks(tasks: &LinkedList<Arc<Spinlock<Task>>>, title: &'static str) {
        crate::println!("{title} tasks");
        for task in tasks {
            if let Some(guard) = task.try_acquire() {
                guard.print();
            } else {
                crate::println!("task was locked");
            }
        }
    }

    pub fn print_state(&self) {
        use crate::println;

        println!("\nprinting schedule state: \n");

        self.current_task
            .as_ref()
            .inspect(|task| task.acquire().print());
        Self::print_tasks(&self.ready_tasks, "ready");
        Self::print_tasks(&self.blocked_tasks, "blocked");

        println!("time_slice: {}", self.time_slice);
    }

    pub fn is_idle(&self) -> bool {
        self.current_task
            .clone()
            .is_some_and(|task| task.acquire().common_name == "idle")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockedReason {
    Paused,
    WaitingForMutex,
    SleepingUntil(Duration),
    Special(SpecialCases),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialCases {
    Cleaner,
}

#[derive(Debug, PartialEq, Eq)]
pub enum State {
    Running,
    ReadyToRun,
    Blocked(BlockedReason),
    Dead,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct TaskID(pub u64);

#[derive(Debug)]
#[repr(C)]
pub struct Registers {
    rbx: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,
}

#[derive(Debug)]
#[repr(C)]
pub struct Task {
    pub stack: VirtAddr,     // rsp
    pub stack_top: VirtAddr, //rbp
    registers: Registers,
    pub cr3: PhysFrame<Size4KiB>,
    pub time_used: Duration,
    pub common_name: String,
    pub state: State,
    pub id: TaskID,
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
    pub fn new(
        common_name: String,
        task_fn: fn() -> !,
        mapper: &mut impl Mapper<Size4KiB>,
        frame_alloc: &mut impl FrameAllocator<Size4KiB>,
    ) -> Self {
        let num_of_stacks = STACK_COUNTER.with_mut_ref(|counter| {
            let temp = *counter;
            *counter += 1;
            temp
        });

        let addr = START_ADDR + 0x1000 * num_of_stacks;

        let stack_page = unsafe { allocate_stack(addr, mapper, frame_alloc) };

        info!("allocated new stack");

        unsafe { Self::crate_new_task(common_name, task_fn, stack_page) }
    }

    pub fn allocate_task(common_name: String, top_of_stack: VirtAddr, stack: VirtAddr) -> Self {
        Self {
            stack,
            stack_top: top_of_stack,
            registers: Registers {
                rbx: 0,
                r12: 0,
                r13: 0,
                r14: 0,
                r15: 0,
            },
            cr3: Cr3::read().0,
            time_used: Duration::new(),
            common_name,
            state: State::ReadyToRun,
            id: TaskID(*STACK_COUNTER.acquire()),
        }
    }

    /// Allocates and insets a new task the linked list
    ///
    /// # Safety
    /// Callers must insure that the stack page is unused
    unsafe fn crate_new_task(
        common_name: String,
        new_task: fn() -> !,
        stack_page: Page<Size4KiB>,
    ) -> Self {
        let end_of_stack_addr = stack_page.start_address() + 0x1000;
        let mut stack_ptr: *mut u64 = end_of_stack_addr.as_mut_ptr();

        let task = Self::allocate_task(common_name, end_of_stack_addr, end_of_stack_addr - (8 * 2));

        // Setup the new stack
        unsafe {
            stack_ptr = stack_ptr.offset(-1);
            *stack_ptr = new_task as u64;
            stack_ptr = stack_ptr.offset(-1);
            *stack_ptr = first_time_task_cleanup as u64;
        }

        task
    }

    fn print(&self) {
        let name = &self.common_name;
        let id = &self.id;
        let state = &self.state;
        let time_used = &self.time_used;
        crate::println!("\tname: {name}\n\tid: {id:?}\n\tstate:{state:?}\n\ttime used:{time_used}");
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
// rbx,  rsp, rbp, r12, r13, r14, r15 need to be saved
#[unsafe(naked)]
pub unsafe extern "sysv64" fn switch_to_task(current_task: *mut Task, next_task: *mut Task) {
    core::arch::naked_asm!(
        "mov [rdi+48], r15",
        "mov [rdi+40], r14",
        "mov [rdi+32], r13",
        "mov [rdi+24], r12",
        "mov [rdi+16], rbx",
        "mov [rdi+8], rbp", // store rbp
        "mov [rdi], rsp",   // store rsp in task struct
        "mov rsp, [rsi]",   // load rsp from the next task
        "mov rbp, [rsi+8]", //load rbp
        "mov rbx, [rsi+16]",
        "mov r12, [rsi+24]",
        "mov r13, [rsi+32]",
        "mov r14, [rsi+40]",
        "mov r15, [rsi+48]",
        "ret",
    );
}

fn first_time_task_cleanup() {
    x86_64::instructions::interrupts::enable();
    SCHEDULER.release();
}

fn get_time_elapsed() -> Duration {
    TIME_KEEPER.with_mut_ref(|keeper| {
        let elep = keeper.schedule_counter.time;
        keeper.schedule_counter.time.reset();
        elep
    })
}

/// Schedule and switches to the next task.
///
/// # Safety
///
/// SCHEDULER spinlock must not be held when called.
pub unsafe fn schedule() {
    without_interrupts(|| {
        let elapsed = get_time_elapsed();

        SCHEDULER.with_mut_ref(|scheduler| {
            if !scheduler.ready_tasks.is_empty() {
                let current_task = scheduler.current_task.take().unwrap();
                let next_task = scheduler.ready_tasks.pop_front().unwrap();

                let current_task_ptr = current_task.with_mut_ref(|task| {
                    task.time_used += elapsed;
                    task.state = State::ReadyToRun;

                    core::ptr::from_mut(task)
                });

                let next_task_ptr = next_task.with_mut_ref(|task| {
                    task.state = State::Running;

                    core::ptr::from_mut(task)
                });

                scheduler.current_task.replace(next_task);
                scheduler.ready_tasks.push_back(current_task);
                scheduler.time_slice = Scheduler::TIME_SLICE_AMOUNT;

                unsafe {
                    switch_to_task(current_task_ptr, next_task_ptr);
                }
            }
        });
    });
}

/// Blocks the current task.
///
/// # Safety
///
/// Scheduler and time keeper must not be held.
pub unsafe fn block_task(reason: BlockedReason) {
    without_interrupts(|| {
        let elapsed = get_time_elapsed();

        SCHEDULER.with_mut_ref(|scheduler| {
            if !scheduler.ready_tasks.is_empty() {
                // TODO: replace with unreachable
                let current_task = scheduler.current_task.take().unwrap();
                let next_task = scheduler.ready_tasks.pop_front().unwrap();

                let current_task_ptr = current_task.with_mut_ref(|task| {
                    task.time_used += elapsed;
                    task.state = State::Blocked(reason);

                    core::ptr::from_mut(task)
                });

                let next_task_ptr = next_task.with_mut_ref(|task| {
                    task.state = State::Running;

                    core::ptr::from_mut(task)
                });

                match reason {
                    BlockedReason::Special(special_cases) => match special_cases {
                        SpecialCases::Cleaner => {
                            scheduler.cleaner_task.replace(current_task);
                        }
                    },
                    _ => {
                        scheduler.blocked_tasks.push_front(current_task);
                    }
                }

                scheduler.current_task.replace(next_task);
                scheduler.time_slice = Scheduler::TIME_SLICE_AMOUNT;

                unsafe { switch_to_task(current_task_ptr, next_task_ptr) };
            }
        });
    });
}

#[derive(thiserror::Error, Debug)]
pub enum UnblockingError {
    #[error("Unable to find any blocked tasks with a matching id")]
    FailedToFindBlockedTask,
}

/// Unblocks the given tasks.
///
/// # Safety
///
/// Scheduler and time keeper spin lock must not be held.
pub unsafe fn unblock_task(task: Arc<Spinlock<Task>>) {
    without_interrupts(|| {
        let elapsed = get_time_elapsed();

        SCHEDULER.with_mut_ref(|scheduler| {
            // TODO: replace with unreachable
            let current_task = scheduler.current_task.take().unwrap();
            let next_task = task;

            let current_task_ptr = current_task.with_mut_ref(|task| {
                task.time_used += elapsed;
                task.state = State::ReadyToRun;

                core::ptr::from_mut(task)
            });

            let next_task_ptr = next_task.with_mut_ref(|task| {
                task.state = State::Running;
                debug!("unblocking task {}", task.common_name);

                core::ptr::from_mut(task)
            });

            // no tasks were running before hand so preempt
            if scheduler.ready_tasks.is_empty() {
                scheduler.current_task.replace(next_task);
                scheduler.ready_tasks.push_back(current_task);
                scheduler.time_slice = Scheduler::TIME_SLICE_AMOUNT;

                unsafe {
                    switch_to_task(current_task_ptr, next_task_ptr);
                }
            } else {
                scheduler.current_task.replace(current_task);
                scheduler.ready_tasks.push_back(next_task);
            }
        });
    });
}

/// This function will unblock the task wit the associated id.
///
/// # Errors
///
/// This function will return an error if no matching task can be found.
///
/// # Safety
///
/// Scheduler and time keeper must not be held.
pub unsafe fn unblock_task_id(task_id: TaskID) -> Result<(), UnblockingError> {
    let task = SCHEDULER.with_mut_ref(|scheduler| {
        scheduler
            .blocked_tasks
            .extract_if(|b_task| b_task.acquire().id == task_id)
            .next()
            .ok_or(UnblockingError::FailedToFindBlockedTask)
    })?;

    assert!(!TIME_KEEPER.is_acquired());
    assert!(!SCHEDULER.is_acquired());

    // SAFETY: Asserts assure both spinlocks are not locked
    unsafe { unblock_task(task) };

    Ok(())
}

pub fn sleep(duration: Duration) {
    let instant = TIME_KEEPER
        .with_ref(|time| time.time_since_boot.time)
        .get_nanoseconds();

    assert!(!SCHEDULER.is_acquired());

    // SAFETY: Scheduler locked is checked by assert
    unsafe {
        block_task(BlockedReason::SleepingUntil(duration + instant));
    }
}

/// Terminate the current task and wakes up the cleaner.
///
/// # Safety
///
/// [`SCHEDULER`] must not be held.
pub unsafe fn exit() -> ! {
    without_interrupts(|| {
        SCHEDULER.with_mut_ref(|scheduler| {
            // TODO: replace with unreachable
            let current_task = scheduler.current_task.take().unwrap();
            let next_task = scheduler.ready_tasks.pop_front().unwrap();

            let current_task_ptr = current_task.with_mut_ref(|task| {
                task.state = State::Dead;

                core::ptr::from_mut(task)
            });

            let next_task_ptr = next_task.with_mut_ref(|task| {
                task.state = State::Running;

                core::ptr::from_mut(task)
            });

            scheduler.current_task.replace(next_task);
            scheduler.dead_tasks.push_back(current_task);
            scheduler.time_slice = Scheduler::TIME_SLICE_AMOUNT;

            // If cleaner is not in it's field it must already be ready
            if let Some(cleaner) = scheduler.cleaner_task.take() {
                scheduler.ready_task(cleaner);
            }

            unsafe {
                switch_to_task(current_task_ptr, next_task_ptr);
            }
        });
    });

    panic!("this should be unreachable");
}

fn cleanup_task(_task: Task) {
    //TODO to free the stack page used;
}

fn cleaner_task() -> ! {
    loop {
        SCHEDULER.with_mut_ref(|scheduler| {
            while let Some(task) = scheduler.dead_tasks.pop_front() {
                let task = Arc::into_inner(task).unwrap();
                let task = task.into_inner();
                info!("killing task {}", task.common_name);

                cleanup_task(task);
            }

            info!("done killing tasks, going to sleep");
        });

        unsafe { block_task(BlockedReason::Special(SpecialCases::Cleaner)) };
    }
}

fn idle_task() -> ! {
    hlt_loop()
}
