use crate::filesystem::FileTrait;
use crate::gdt::TSS;
use crate::timer::{Duration, Miliseconds, TIME_KEEPER, TimeKeeper};
use crate::usermode::{into_usermode, load_elf};
use crate::{P_OFFSET, hlt_loop, memory};
use alloc::collections::linked_list::LinkedList;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use log::{debug, info};
use spinlock::Spinlock;
use x86_64::instructions::interrupts::without_interrupts;
use x86_64::structures::paging::{
    FrameAllocator, Mapper, OffsetPageTable, Page, PageTableFlags, Size4KiB,
};
use x86_64::{PhysAddr, VirtAddr};

// pub mod mutex;
pub mod mutex {
    pub type Mutex<T> = spinlock::Spinlock<T>;
}

// TEMP, this is not checked big UB!
// Setup some way to track used pages
static STACK_COUNTER: Spinlock<u64> = Spinlock::new(0);

static KERNEL_STACK: Spinlock<VirtAddr> = Spinlock::new(VirtAddr::new(0xFFFF_E000_0000_0000));

static USER_STACK: Spinlock<VirtAddr> = Spinlock::new(VirtAddr::new(0x0000_0000_F000_0000));

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
        let cleaner_task = Task::new_kernel(
            String::from("Cleaner"),
            cleaner_task as *const () as u64,
            mapper,
            frame_alloc,
        );
        let idle_task = Task::new_kernel(
            String::from("idle"),
            idle_task as *const () as u64,
            mapper,
            frame_alloc,
        );

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
    pub kernel_stack: VirtAddr,     // rsp
    pub kernel_stack_top: VirtAddr, //rbp
    registers: Registers,
    pub cr3_paddr: PhysAddr,
    pub time_used: Duration,
    pub common_name: String,
    pub state: State,
    pub id: TaskID,
    user_stack_top: Option<VirtAddr>,
    entry: Option<u64>,
}

unsafe impl Send for Task {}

#[derive(thiserror::Error, Debug)]
pub enum TaskBuildError {
    #[error("The first task was not crated before trying to crate new tasks")]
    MissingFirstTaskBeforeAllocatingNewTask,
}

impl Task {
    pub fn new_usermode(
        file: &dyn FileTrait,
        kernel_mapper: &mut impl Mapper<Size4KiB>,
        common_name: String,
        frame_alloc: &mut impl FrameAllocator<Size4KiB>,
    ) -> Self {
        let (page, frame) = memory::new_table(frame_alloc, VirtAddr::new(P_OFFSET));

        let mut mapper = unsafe { OffsetPageTable::new(page, VirtAddr::new(P_OFFSET)) };

        let entry = load_elf(file, &mut mapper, frame_alloc).unwrap();

        let kernel_stack = allocate_kernel_stack(kernel_mapper, frame_alloc);

        let user_stack = allocate_user_stack(&mut mapper, frame_alloc);

        let end_of_stack_addr = kernel_stack[3].start_address() + 0x1000;
        let mut stack_ptr: *mut u64 = end_of_stack_addr.as_mut_ptr();

        // Setup the new stack
        unsafe {
            stack_ptr = stack_ptr.sub(1);
            *stack_ptr = entry;
            stack_ptr = stack_ptr.sub(1);
            *stack_ptr = first_time_task_cleanup as *const () as u64;
        }

        Self {
            kernel_stack: end_of_stack_addr - (8 * 2),
            kernel_stack_top: end_of_stack_addr,
            user_stack_top: Some(user_stack[3].start_address() + 0x1000),
            entry: Some(entry),
            registers: Registers {
                rbx: 0,
                r12: 0,
                r13: 0,
                r14: 0,
                r15: 0,
            },
            cr3_paddr: frame.start_address(),
            time_used: Duration::new(),
            common_name,
            state: State::ReadyToRun,
            id: TaskID(*STACK_COUNTER.acquire()),
        }
    }

    /// Allocates and insets a new task the linked list
    pub fn new_kernel(
        common_name: String,
        task_fn: u64,
        mapper: &mut impl Mapper<Size4KiB>,
        frame_alloc: &mut impl FrameAllocator<Size4KiB>,
    ) -> Self {
        let (_, frame) = memory::new_table(frame_alloc, VirtAddr::new(P_OFFSET));

        let kernel_stack = allocate_kernel_stack(mapper, frame_alloc);

        let end_of_stack_addr = kernel_stack[3].start_address() + 0x1000;
        let mut stack_ptr: *mut u64 = end_of_stack_addr.as_mut_ptr();

        // Setup the new stack
        unsafe {
            stack_ptr = stack_ptr.sub(1);
            *stack_ptr = task_fn;
            stack_ptr = stack_ptr.sub(1);
            *stack_ptr = first_time_task_cleanup as *const () as u64;
        }

        Self {
            kernel_stack: end_of_stack_addr - (8 * 2),
            kernel_stack_top: end_of_stack_addr,
            user_stack_top: None,
            entry: None,
            registers: Registers {
                rbx: 0,
                r12: 0,
                r13: 0,
                r14: 0,
                r15: 0,
            },
            cr3_paddr: frame.start_address(),
            time_used: Duration::new(),
            common_name,
            state: State::ReadyToRun,
            id: TaskID(*STACK_COUNTER.acquire()),
        }
    }

    pub fn allocate_task(
        common_name: String,
        top_of_stack: VirtAddr,
        kernel_stack: VirtAddr,
        frame_alloc: &mut impl FrameAllocator<Size4KiB>,
    ) -> Self {
        let (_, frame) = memory::new_table(frame_alloc, VirtAddr::new(P_OFFSET));

        Self {
            kernel_stack,
            kernel_stack_top: top_of_stack,
            user_stack_top: None,
            entry: None,
            registers: Registers {
                rbx: 0,
                r12: 0,
                r13: 0,
                r14: 0,
                r15: 0,
            },
            cr3_paddr: frame.start_address(),
            time_used: Duration::new(),
            common_name,
            state: State::ReadyToRun,
            id: TaskID(*STACK_COUNTER.acquire()),
        }
    }

    fn print(&self) {
        let name = &self.common_name;
        let id = &self.id;
        let state = &self.state;
        let time_used = &self.time_used;
        crate::println!("\tname: {name}\n\tid: {id:?}\n\tstate:{state:?}\n\ttime used:{time_used}");
    }
}

fn allocate_user_stack(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_alloc: &mut impl FrameAllocator<Size4KiB>,
) -> [Page<Size4KiB>; 4] {
    let mut addr = USER_STACK.acquire();

    let flags =
        PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;

    allocate_stack_with_flags(mapper, frame_alloc, flags, &mut addr)
}

fn allocate_kernel_stack(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_alloc: &mut impl FrameAllocator<Size4KiB>,
) -> [Page<Size4KiB>; 4] {
    let mut addr = KERNEL_STACK.acquire();

    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

    allocate_stack_with_flags(mapper, frame_alloc, flags, &mut addr)
}

fn allocate_stack_with_flags(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_alloc: &mut impl FrameAllocator<Size4KiB>,
    flags: PageTableFlags,
    addr: &mut VirtAddr,
) -> [Page<Size4KiB>; 4] {
    let pages: [Page<Size4KiB>; 4] = [
        kernel_stack_page(addr),
        kernel_stack_page(addr),
        kernel_stack_page(addr),
        kernel_stack_page(addr),
    ];

    // gaurd page
    let _ = kernel_stack_page(addr);

    for page in &pages {
        let frame = frame_alloc.allocate_frame().unwrap();
        unsafe {
            mapper
                .map_to(*page, frame, flags, frame_alloc)
                .unwrap()
                .ignore();
        }
    }

    pages
}

fn kernel_stack_page(addr: &mut VirtAddr) -> Page<Size4KiB> {
    let page = Page::from_start_address(*addr).unwrap();

    *addr += 0x1000;

    page
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
            .map_to(page_stack, stack_frame, stack_flags, frame_alloc)
            .unwrap()
            .flush();
    }

    page_stack
}

#[allow(clippy::needless_pass_by_value)]
/// Switch to the next task in the linked list
///
/// # Safety
/// Callers must insure that interrupts are disabled while this function is being called.
/// `current_task` must also be the current task
pub unsafe fn switch_to_task(current_task: Arc<Spinlock<Task>>, next_task: Arc<Spinlock<Task>>) {
    let current_task_ptr = current_task.with_mut_ref(core::ptr::from_mut);

    let next_task_ptr = next_task.with_mut_ref(|task| {
        unsafe {
            TSS.privilege_stack_table[0] = task.kernel_stack_top;
            crate::syscalls::KERNEL_RSP.1 = task.kernel_stack_top.as_u64();
        }

        core::ptr::from_mut(task)
    });

    drop(current_task);
    drop(next_task);

    unsafe { switch_to_task_inner(current_task_ptr, next_task_ptr) };
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
pub unsafe extern "sysv64" fn switch_to_task_inner(current_task: *mut Task, next_task: *mut Task) {
    core::arch::naked_asm!(
        "mov [rdi+48], r15",
        "mov r15, [rsi+48]",
        "mov [rdi+40], r14",
        "mov r14, [rsi+40]",
        "mov [rdi+32], r13",
        "mov r13, [rsi+32]",
        "mov [rdi+24], r12",
        "mov r12, [rsi+24]",
        "mov [rdi+16], rbx",
        // "mov rbx, [rsi+16]",
        "mov [rdi+8], rbp", // store rbp
        "mov rbp, [rsi+8]", //load rbp
        "mov [rdi], rsp",   // store rsp in task struct
        "mov rsp, [rsi]",   // load rsp from the next task
        "mov rbx, cr3",     // change virtual address spaces
        "mov [rdi+56], rbx",
        "mov rbx, [rsi+16]",
        "push rbx",
        "mov rbx, [rsi+56]",
        "mov cr3, rbx",
        "pop rbx",
        "ret",
    );
}

fn first_time_task_cleanup() {
    x86_64::instructions::interrupts::enable();
    SCHEDULER.release();

    let sched = SCHEDULER.acquire();

    let current = sched.current_task.as_ref().unwrap();

    #[allow(clippy::branches_sharing_code)]
    if let Some((entry, stack)) = current.with_ref(|task| task.entry.zip(task.user_stack_top)) {
        drop(sched);
        unsafe {
            into_usermode(entry, stack.as_u64());
        }
    } else {
        drop(sched);
    }
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

                current_task.with_mut_ref(|task| {
                    task.time_used += elapsed;
                    task.state = State::ReadyToRun;
                });

                next_task.with_mut_ref(|task| {
                    task.state = State::Running;
                });

                scheduler.current_task.replace(next_task.clone());
                scheduler.ready_tasks.push_back(current_task.clone());
                scheduler.time_slice = Scheduler::TIME_SLICE_AMOUNT;

                unsafe {
                    switch_to_task(current_task, next_task);
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
    // log::trace!("blocking");
    without_interrupts(|| {
        let elapsed = get_time_elapsed();

        SCHEDULER.with_mut_ref(|scheduler| {
            if !scheduler.ready_tasks.is_empty() {
                // TODO: replace with unreachable
                let current_task = scheduler.current_task.take().unwrap();
                let next_task = scheduler.ready_tasks.pop_front().unwrap();

                current_task.with_mut_ref(|task| {
                    task.time_used += elapsed;
                    task.state = State::Blocked(reason);
                });

                next_task.with_mut_ref(|task| {
                    task.state = State::Running;
                });

                match reason {
                    BlockedReason::Special(special_cases) => match special_cases {
                        SpecialCases::Cleaner => {
                            scheduler.cleaner_task.replace(current_task.clone());
                        }
                    },
                    _ => {
                        scheduler.blocked_tasks.push_front(current_task.clone());
                    }
                }

                scheduler.current_task.replace(next_task.clone());
                scheduler.time_slice = Scheduler::TIME_SLICE_AMOUNT;

                unsafe { switch_to_task(current_task, next_task) };
            }
        });
    });

    // log::trace!("unblocking");
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

            current_task.with_mut_ref(|task| {
                task.time_used += elapsed;
                task.state = State::ReadyToRun;
            });

            next_task.with_mut_ref(|task| {
                task.state = State::Running;
                debug!("unblocking task {}", task.common_name);
            });

            // no tasks were running before hand so preempt
            if scheduler.ready_tasks.is_empty() {
                scheduler.current_task.replace(next_task.clone());
                scheduler.ready_tasks.push_back(current_task.clone());
                scheduler.time_slice = Scheduler::TIME_SLICE_AMOUNT;

                unsafe {
                    switch_to_task(current_task, next_task);
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

            current_task.with_mut_ref(|task| {
                task.state = State::Dead;
            });

            next_task.with_mut_ref(|task| {
                task.state = State::Running;
            });

            scheduler.current_task.replace(next_task.clone());
            scheduler.dead_tasks.push_back(current_task.clone());
            scheduler.time_slice = Scheduler::TIME_SLICE_AMOUNT;

            // If cleaner is not in it's field it must already be ready
            if let Some(cleaner) = scheduler.cleaner_task.take() {
                scheduler.ready_task(cleaner);
            }

            unsafe {
                switch_to_task(current_task, next_task);
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
