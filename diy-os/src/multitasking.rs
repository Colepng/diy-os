use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::timer::sleep;

const LOOP_TIME: u64 = 3; // length of loop in ms

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

            sleep(LOOP_TIME.saturating_add(took));
        }
    }
}

pub trait Task {
    fn run(&mut self);
}
