use alloc::boxed::Box;
use alloc::vec::Vec;

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
            self.tasks.iter_mut().for_each(|task| task.run());
        }
    }
}

pub trait Task {
    fn run(&mut self);
}
