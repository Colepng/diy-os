use crate::pit;

pub fn setup_system_timer() {
    let mut pit = pit::Pit::take().expect("Something else has control over the pit");

    let configure_command = pit::ConfigureChannelCommand::new(
        pit::Channel::Channel0,
        pit::AccessMode::LowHighbyte,
        pit::OperatingMode::SquareWaveGenerator,
        pit::BcdBinaryMode::Binary16Bit,
    );

    pit.mode_port.write(configure_command);

    let divider = unsafe { pit::Pit::frequency_divder_from_frequency_unchecked(100) };

    pit.set_frequency_divder(divider);
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
