use crate::spinlock::Spinlock;
use alloc::boxed::Box;
use controller::Inital;
pub use controller::controllers::generic::GenericPS2Controller;
use devices::PS2Device;

pub mod controller;
pub mod devices;

pub static CONTROLLER: Spinlock<Option<GenericPS2Controller<Inital>>> = Spinlock::new(None);
pub static PS1_DEVICE: Spinlock<Option<Box<dyn PS2Device + Send + Sync>>> = Spinlock::new(None);
