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

/// time in ms
pub fn sleep(count: u64) {
    *pit::SLEEP_COUNTER.acquire() = count;
    pit::SLEEP_COUNTER.release();

    while *pit::SLEEP_COUNTER.acquire() > 0 {
        pit::SLEEP_COUNTER.release();
        x86_64::instructions::hlt();
    }
}
