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

pub struct TimeKeeper {
    pub sleep_counter: u64,
    pub timer_counter: u64,
    pub keyboard_counter: u64,
}

impl TimeKeeper {
    pub const fn new() -> Self {
        Self {
            sleep_counter: 0,
            timer_counter: 0,
            keyboard_counter: 0,
        }
    }

    pub const fn tick(&mut self) {
        self.sleep_counter = self.sleep_counter.saturating_sub(1);
        self.timer_counter = self.timer_counter.saturating_add(1);
        self.keyboard_counter = self.keyboard_counter.saturating_add(1);
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

pub fn time<F, R>(f: F) -> (R, u64)
where
    F: FnOnce() -> R,
{
    TIME_KEEPER.acquire().timer_counter = 0;
    TIME_KEEPER.release();

    let ret = f();

    let ms = TIME_KEEPER.acquire().timer_counter;
    TIME_KEEPER.release();

    (ret, ms)
}
