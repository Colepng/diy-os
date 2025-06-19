#![no_std]
#![warn(
    clippy::pedantic,
    clippy::nursery,
    clippy::perf,
    clippy::style,
    clippy::todo,
)]
#![deny(
    clippy::suspicious,
    clippy::correctness,
    clippy::complexity,
    clippy::missing_const_for_fn,
    unsafe_code
)]

use crate::alloc::string::ToString;
use log::Level;

extern crate alloc;

/// Initializes the logger for use with the log library
///
/// # Errors
///
/// This function will return an error if the logger was already initialized.
pub fn init(store: fn(&'static str, Level)) -> Result<(), log::SetLoggerError> {
    use alloc::boxed::Box;
    let logger = Box::new(Logger::new(store));

    log::set_logger(Box::leak(logger))?;
    log::set_max_level(log::LevelFilter::Trace);

    Ok(())
}

pub struct Logger {
    store: fn(&'static str, Level) -> (),
}

impl Logger {
    pub const fn new(store: fn(&'static str, Level)) -> Self {
        Self {
            store
        }
    }
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= Level::Trace
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let string = record.args().as_str().or_else(|| {
                let string = record.args().to_string();
                Some(string.leak())
            }).unwrap();
            (self.store)(string, record.level());
        }
    }

    fn flush(&self) {}
}
