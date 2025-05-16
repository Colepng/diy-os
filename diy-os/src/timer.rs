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
pub struct Seconds(pub u64);

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct Miliseconds(pub u64);

impl<T: Into<Self>> core::ops::Add<T> for Miliseconds {
    type Output = Self;

    fn add(self, rhs: T) -> Self::Output {
        Self(self.0 + rhs.into().0)
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct Microseconds(pub u64);

impl<T: Into<Self>> core::ops::Add<T> for Microseconds {
    type Output = Self;

    fn add(self, rhs: T) -> Self::Output {
        Self(self.0 + rhs.into().0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Nanoseconds(pub u64);

impl<T: Into<Self>> core::ops::Add<T> for Nanoseconds {
    type Output = Self;

    fn add(self, rhs: T) -> Self::Output {
        Self(self.0 + rhs.into().0)
    }
}

impl<T: Into<Self>> core::ops::Sub<T> for Nanoseconds {
    type Output = Self;

    fn sub(self, rhs: T) -> Self::Output {
        Self(self.0 - rhs.into().0)
    }
}

impl From<Nanoseconds> for Seconds {
    fn from(value: Nanoseconds) -> Self {
        Self(value.0/1_000_000_000)
    }
}

impl From<Nanoseconds> for Miliseconds {
    fn from(value: Nanoseconds) -> Self {
        Self(value.0/1_000_000)
    }
}

impl From<Nanoseconds> for Microseconds {
    fn from(value: Nanoseconds) -> Self {
        Self(value.0/1_000)
    }
}


impl From<Seconds> for Nanoseconds {
    fn from(value: Seconds) -> Self {
        Self(value.0 * 1_000_000_000)
    }
}

impl From<Miliseconds> for Nanoseconds {
    fn from(value: Miliseconds) -> Self {
        Self(value.0 * 1_000_000)
    }
}

impl From<Microseconds> for Nanoseconds {
    fn from(value: Microseconds) -> Self {
        Self(value.0 * 1_000)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Time {
    nanoseconds: Nanoseconds,
}

impl<T: Into<Nanoseconds>> From<T> for Time {
    fn from(value: T) -> Self {
        Self { nanoseconds: value.into() }
    }
}

impl<T: Into<Nanoseconds>> core::ops::Add<T> for Time {
    type Output = Self;

    fn add(self, rhs: T) -> Self::Output {
        Self { nanoseconds: self.nanoseconds + rhs }
    }
}

impl core::ops::AddAssign for Time {
    fn add_assign(&mut self, rhs: Self) {
        self.nanoseconds = self.nanoseconds + rhs.nanoseconds;
    }
}

impl core::fmt::Display for Time {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let seconds: Seconds = self.nanoseconds.into();
        let milis: Miliseconds = (self.nanoseconds - seconds).into(); 
        let micros: Microseconds = (self.nanoseconds - seconds - milis).into();
        f.write_fmt(format_args!("{}:{}:{}:{}", seconds.0, milis.0, micros.0, (self.nanoseconds - seconds - milis - micros).0))
    }
}

impl Time {
    pub const ZERO: Self = Self::new();
    
    pub const fn new() -> Self {
        Self {
            nanoseconds: Nanoseconds(0)
        }
    }

    pub const fn reset(&mut self) {
        self.nanoseconds.0 = 0;
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
    pub tick_amount: Nanoseconds,
    pub sleep_counter: u64,
    pub timer_counter: Counter,
    pub keyboard_counter: Counter,
    pub log_counter: Counter,
    pub schedule_counter: Counter,
}

impl TimeKeeper {
    pub const fn new() -> Self {
        Self {
            tick_amount: Nanoseconds(1_000_000),
            sleep_counter: 0,
            timer_counter: Counter::new(),
            keyboard_counter: Counter::new(),
            log_counter: Counter::new(),
            schedule_counter: Counter::new(),
        }
    }

    pub fn tick(&mut self) {
        self.sleep_counter = self.sleep_counter.saturating_sub(1);
        self.timer_counter.time += self.tick_amount.into();
        self.keyboard_counter.time += self.tick_amount.into();
        self.log_counter.time += self.tick_amount.into();
        self.schedule_counter.time += self.tick_amount.into();
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
