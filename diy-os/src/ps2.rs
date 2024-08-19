use crate::spinlock::Spinlock;
use controllers::gernaric::Generic;

pub mod controllers;

pub static CONTROLLER: Spinlock<Option<Generic>> = Spinlock::new(None);
