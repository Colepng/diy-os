// use keyboard::Command;

use crate::multitasking::schedule;

pub mod keyboard;

pub trait PS2Device {
    // fn add_command(&moot self, value: Command);
    fn received_byte(&mut self, byte: u8);

    fn periodic(&mut self);
}

pub fn ps2_device_1_task() -> ! {
    x86_64::instructions::interrupts::enable();

    loop {
        super::PS1_DEVICE.with_mut_ref(|device| device.as_mut().unwrap().periodic());

        schedule();
    }
}
