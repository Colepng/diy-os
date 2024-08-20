use crate::spinlock::Spinlock;
pub use controller::controllers::generic::GenericPS2Controller;
use controller::Waiting;

pub mod controller;

pub static CONTROLLER: Spinlock<Option<GenericPS2Controller<Waiting>>> = Spinlock::new(None);
