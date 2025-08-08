use alloc::boxed::Box;

use crate::{
    multitasking::sleep,
    ps2::{
        CONTROLLER, GenericPS2Controller, controller::PS2Controller, devices::keyboard::Keyboard,
    },
    timer::{Duration, Miliseconds},
};

pub mod keyboard;

pub trait PS2Device {
    // fn add_command(&moot self, value: Command);
    fn received_byte(&mut self, byte: u8);

    fn periodic(&mut self);
}

pub fn ps2_device_1_task() -> ! {
    let generic = GenericPS2Controller::new().initialize();
    CONTROLLER.with_mut_ref(|controller| controller.replace(generic));

    super::PS1_DEVICE.with_mut_ref(|ps1| ps1.replace(Box::new(Keyboard::new())));
    loop {
        super::PS1_DEVICE.with_mut_ref(|device| device.as_mut().unwrap().periodic());
        sleep(Duration::from(Miliseconds(10)));
    }
}
