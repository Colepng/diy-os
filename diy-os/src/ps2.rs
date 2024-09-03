use crate::spinlock::Spinlock;
pub use controller::controllers::generic::GenericPS2Controller;
use controller::Inital;

pub mod controller;

pub static CONTROLLER: Spinlock<Option<GenericPS2Controller<Inital>>> = Spinlock::new(None);
