// use keyboard::Command;

use crate::multitasking::Task;

pub mod keyboard;

pub trait PS2Device {
    // fn add_command(&moot self, value: Command);
    fn received_byte(&mut self, byte: u8);

    fn periodic(&mut self);
}

pub struct PS2Device1Task;

impl Task for PS2Device1Task {
    fn run(&mut self) {
        let mut device = super::PS1_DEVICE.acquire();
        let device = device.as_mut().unwrap();

        device.periodic();
    }
}
