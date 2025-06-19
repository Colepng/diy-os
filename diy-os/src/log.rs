use core::fmt::Display;

use alloc::vec::Vec;

use crate::timer::{Duration, TIME_KEEPER};
use spinlock::Spinlock;

pub static LOGGER: Spinlock<Logger> = Spinlock::new(Logger::new());

pub fn error(message: &'static str) {
    LOGGER.with_mut_ref(|logger| {
        logger.error(message);
    });
}

pub fn warn(message: &'static str) {
    LOGGER.with_mut_ref(|logger| {
        logger.warn(message);
    });
}

pub fn info(message: &'static str) {
    LOGGER.with_mut_ref(|logger| {
        logger.info(message);
    });
}

pub fn debug(message: &'static str) {
    LOGGER.with_mut_ref(|logger| {
        logger.debug(message);
    });
}

pub fn trace(message: &'static str) {
    LOGGER.with_mut_ref(|logger| {
        logger.trace(message);
    });
}

pub struct Logger {
    events: Vec<Event>,
}

impl Logger {
    pub const fn new() -> Self {
        Self { events: Vec::new() }
    }

    fn log(&mut self, text: &'static str, level: LogLevel) {
        let event = Event {
            text,
            time: TIME_KEEPER.with_ref(|keeper| keeper.log_counter.time),
            level,
        };

        self.events.push(event);
    }

    pub fn error(&mut self, text: &'static str) {
        self.log(text, LogLevel::Error);
    }

    pub fn warn(&mut self, text: &'static str) {
        self.log(text, LogLevel::Warn);
    }

    pub fn info(&mut self, text: &'static str) {
        self.log(text, LogLevel::Info);
    }

    pub fn debug(&mut self, text: &'static str) {
        self.log(text, LogLevel::Debug);
    }

    pub fn trace(&mut self, text: &'static str) {
        self.log(text, LogLevel::Trace);
    }

    pub fn get_events(&self) -> impl Iterator<Item = &Event> {
        self.events.iter()
    }
}

pub struct Event {
    text: &'static str,
    time: Duration,
    pub level: LogLevel,
}

impl Display for Event {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!(
            "{}, {:?}: {}",
            self.time, self.level, self.text
        ))
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}
