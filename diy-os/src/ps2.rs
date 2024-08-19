use crate::spinlock::Spinlock;
pub use controller::controllers::generic::GenericPS2Controller;

pub mod controller;

pub static CONTROLLER: Spinlock<Option<GenericPS2Controller>> = Spinlock::new(None);
