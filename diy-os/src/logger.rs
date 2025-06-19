use core::fmt::Display;

use alloc::vec::Vec;
use log::Level;

use crate::timer::{Duration, TIME_KEEPER};
use spinlock::Spinlock;

pub static LOGGER: Spinlock<Logger> = Spinlock::new(Logger::new());

pub fn store(text: &'static str, level: Level) {
    LOGGER.with_mut_ref(|logger| {
        let event = Event {
            text,
            time: TIME_KEEPER.with_ref(|keeper| keeper.log_counter.time),
            level,
        };
        logger.events.push(event);
    });
}
pub struct Logger {
    events: Vec<Event>,
}

impl Logger {
    pub const fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn get_events(&self) -> impl Iterator<Item = &Event> {
        self.events.iter()
    }
}

pub struct Event {
    text: &'static str,
    time: Duration,
    pub level: Level,
}

impl Display for Event {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!(
            "{}, {:?}: {}",
            self.time, self.level, self.text,
        ))
    }
}
