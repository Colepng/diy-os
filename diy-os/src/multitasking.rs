use alloc::vec::Vec;
use lazy_static::lazy_static;
use x86_64::VirtAddr;

use crate::{println, spinlock::Spinlock, timer};

pub mod usermode;

lazy_static!(
pub static ref SCHEDULER: Spinlock<ProcessTracker> = { 

Spinlock::new(ProcessTracker::new())
};
    );

pub struct ProcessTracker {
    processes: Vec<Process>,
    counter: timer::CounterIndex,
    pub index: usize,
}

// in number of 10 miliseconds
// 100 total
const QUANTUM: u64 = 10;

impl ProcessTracker {
    pub fn new() -> Self {
        Self {
            processes: Vec::new(),
            counter: timer::TIMEKEEPER.acquire().new_decrementing_counter(QUANTUM, next_process),
            index: 0,
        }
    }

    pub fn spawn_new(&mut self, process: Process) {
        self.processes.push(process);
    }

    pub fn process_finished(&mut self, index: usize) {
        self.processes.remove(index);
        if self.index > index {
            self.index -= 1;
        }
    }
}

pub fn switch_to_task() {
    let scheduler = SCHEDULER.acquire();

    if !scheduler.processes.is_empty() {
        println!("not empty");
        let process = &scheduler.processes[scheduler.index];
        let process_entry = process.entry;
        let process_stack = process.stack;

        core::mem::drop(scheduler);

        usermode::into_usermode(process_entry.as_u64(), process_stack.as_u64());
    } else {
        core::mem::drop(scheduler);
    }

    x86_64::instructions::hlt();
}

pub fn next_process() {
    let mut scheduler = SCHEDULER.acquire();
    if scheduler.index + 1 >= scheduler.processes.len() {
        scheduler.index = 0;
    } else {
        scheduler.index += 1;
    }
}

pub struct Process {
    stack: VirtAddr,
    entry: VirtAddr,
    // pid: u32,
    // state: ProcessState,
}
impl Process {
    pub fn new(entry: u64, stack: u64) -> Self {
        Self {
            stack: VirtAddr::new(stack),
            entry: VirtAddr::new(entry),
        }
    }
}

enum ProcessState {
    Running,
    Runnable,
    Ended(ExitCode),
    Killed(ExitCode),
    Sleeping,
    Waiting,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum ExitCode {
    Successful = 0,
    Errored = 1,
    /// 128 + signal number
    Unknown,
    // KilledBySignal = 128, 
    OutOfRange = 255,
}

impl From::<u8> for ExitCode {
    fn from(value: u8) -> Self {
        match value {
            0 => ExitCode::Successful,
            1 => ExitCode::Errored,
            // 128..255 => ExitCode::KilledBySignal,
            255 => ExitCode::OutOfRange,
            _ => ExitCode::Unknown,
        }
    }
}
