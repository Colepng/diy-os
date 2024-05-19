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

    let reaload_value = pit::get_reload_value_from_frequency(1000);

    pit::set_count(&mut pit, reaload_value);
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
