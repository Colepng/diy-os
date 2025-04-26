use crate::spinlock::Spinlock;
use crate::{errors::validity::InputOutOfRangeInclusive, pit};

#[derive(thiserror::Error, Debug)]
pub enum SystemTimerError {
    #[error("Frequencny is unsupported")]
    UnsupportedFrequency(#[from] InputOutOfRangeInclusive<u32>),
    #[error("Ownership of system timer could not be accuired")]
    FailedToAccuireOwnsershipOfSystemTimer(),
}

/// # Errors
/// Will return [`SystemTimerError::UnsupportedFrequency`] if `frequency` is unsupported for
/// all system timers.
/// Will return [`SystemTimerError::FailedToAccuireOwnsershipOfSystemTimer`] if all system timers was
/// already owned.
pub fn setup_system_timer(frequency: u32) -> Result<(), SystemTimerError> {
    let mut pit =
        pit::Pit::take().ok_or_else(SystemTimerError::FailedToAccuireOwnsershipOfSystemTimer)?;

    let configure_command = pit::ConfigureChannelCommand::new(
        pit::Channel::Channel0,
        pit::AccessMode::LowHighbyte,
        pit::OperatingMode::SquareWaveGenerator,
        pit::BcdBinaryMode::Binary16Bit,
    );

    pit.mode_port.write(configure_command);

    let freq = pit::PitFrequency::try_new(frequency)?;
    let divider = pit::Pit::frequency_divder_from_frequency(freq);

    pit.set_frequency_divder(divider);

    Ok(())
}

/// Sleep counter
pub static TIME_KEEPER: Spinlock<TimeKeeper> = Spinlock::new(TimeKeeper::new());

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Miliseconds(pub u64);

impl core::ops::Add for Miliseconds {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Seconds(pub u64);

#[derive(Clone, Copy)]
pub struct Time {
    pub miliseconds: Miliseconds,
    pub seconds: Seconds,
}

impl core::fmt::Display for Time {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("{}:{}", self.seconds.0, self.miliseconds.0))
    }
}

impl Time {
    pub const fn new() -> Self {
        Self {
            miliseconds: Miliseconds(0),
            seconds: Seconds(0),
        }
    }
    pub fn add(&mut self, ms: Miliseconds) {
        let mut new_ms = self.miliseconds + ms;
        let seconds = new_ms.0 / 1000;
        self.seconds.0 += seconds;
        new_ms.0 -= seconds * 1000;
        self.miliseconds = new_ms;
    }

    pub const fn reset(&mut self) {
        self.seconds = Seconds(0);
        self.miliseconds = Miliseconds(0);
    }
}

pub struct Counter {
    pub time: Time,
}

impl Counter {
    pub const fn new() -> Self {
        Self {
            time: Time::new(),
        }
    }
}

pub struct TimeKeeper {
    pub tick_amount: Miliseconds,
    pub sleep_counter: u64,
    pub timer_counter: Counter,
    pub keyboard_counter: Counter,
    pub log_counter: Counter, 
}

impl TimeKeeper {
    pub const fn new() -> Self {
        Self {
            tick_amount: Miliseconds(1),
            sleep_counter: 0,
            timer_counter: Counter::new(),
            keyboard_counter: Counter::new(),
            log_counter: Counter::new(),
        }
    }

    pub fn tick(&mut self) {
        self.sleep_counter = self.sleep_counter.saturating_sub(1);
        self.timer_counter.time.add(self.tick_amount);
        self.keyboard_counter.time.add(self.tick_amount);
        self.log_counter.time.add(self.tick_amount);
    }
}

/// time in ms
pub fn sleep(count: u64) {
    TIME_KEEPER.acquire().sleep_counter = count;
    TIME_KEEPER.release();

    while TIME_KEEPER.acquire().sleep_counter > 0 {
        TIME_KEEPER.release();
        x86_64::instructions::hlt();
    }
}

pub fn time<F, R>(f: F) -> (R, Time)
where
    F: FnOnce() -> R,
{
    TIME_KEEPER.acquire().timer_counter.time.reset();
    TIME_KEEPER.release();

    let ret = f();

    let ms = TIME_KEEPER.acquire().timer_counter.time;
    TIME_KEEPER.release();

    (ret, ms)
}
