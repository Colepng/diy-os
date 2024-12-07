use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::timer::sleep;

const LOOP_TIME: u64 = 1; // length of loop in ms

pub struct TaskRunner {
    tasks: Vec<Box<dyn Task>>,
}

impl TaskRunner {
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    pub fn add_task(&mut self, task: impl Task + 'static) {
        self.tasks.push(Box::new(task));
    }

    pub fn start_running(&mut self) -> ! {
        loop {
            let took = crate::timer::time(|| {
                self.tasks.iter_mut().for_each(|task| task.run());
            })
            .1;

            sleep(LOOP_TIME.saturating_add(took.miliseconds.0));
        }
    }
}

pub trait Task {
    fn run(&mut self);
}

pub mod rewrite {
    use core::{arch::asm, mem::MaybeUninit};

    use alloc::boxed::Box;

    #[derive(Debug)]
    #[repr(C)]
    pub struct Task {
        rax: u64,
        pub stack: u64,
    }

    impl Task {
        pub fn new(rax: u64, stack: u64) -> Task {
            Self {
                rax,
                stack,
            }
        }

        pub fn allocate_task(rax: u64, stack: u64) -> Box<Task> {
            let task: Box<MaybeUninit<Task>> = Box::new_uninit();

            unsafe {
                asm!(
                    "mov [{task_ptr}], rax",
                    "mov [{task_ptr}+8], {stack}",
                    task_ptr = in(reg) task.as_ptr(),
                    stack = in(reg) stack,
                    in("rax") rax,
                    )
            }

            return unsafe { task.assume_init() }
        }
    }

}
