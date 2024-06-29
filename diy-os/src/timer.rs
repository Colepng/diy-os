use core::cell::OnceCell;

use alloc::vec::Vec;

use crate::{pit, println, spinlock::Spinlock};

pub static READY: Spinlock<bool> = Spinlock::new(false);

pub fn setup_system_timer() {
    let mut pit = pit::Pit::take().expect("Something else has control over the pit");

    let configure_command = pit::ConfigureChannelCommand::new(
        pit::Channel::Channel0,
        pit::AccessMode::LowHighbyte,
        pit::OperatingMode::SquareWaveGenerator,
        pit::BcdBinaryMode::Binary16Bit,
    );

    pit.mode_port.write(configure_command);

    let reaload_value = pit::get_reload_value_from_frequency(100);

    pit::set_count(&mut pit, reaload_value);

    *READY.acquire() = true;
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

pub static TIMEKEEPER: Spinlock<TimeKeeper> = Spinlock::new(TimeKeeper::new());

pub struct TimeKeeper {
    counters_decrementing: alloc::vec::Vec<(u64, fn())>,
    counters_incermeting: alloc::vec::Vec<u64>,
}

pub enum CounterType {
    Decrementing,
    Incremeting,
}

pub struct CounterIndex {
    index: usize,
    decrementing: bool,
}

impl TimeKeeper {
    pub const fn new() -> Self {
        Self {
            counters_decrementing: Vec::new(),
            counters_incermeting: Vec::new(),
        }
    }

    // Creates a counter that starts at the provided value and decrements until zero
    pub fn new_decrementing_counter(&mut self, start_value: u64, action: fn()) -> CounterIndex {
        self.counters_decrementing.push((start_value, action));

        CounterIndex {
            index: self.counters_decrementing.len() - 1,
            decrementing: true,
        }
    }

    // Creates a counter that starts at 0 and increments infinitely until destroyed
    pub fn new_incrementing_counter(&mut self) -> CounterIndex {
        self.counters_incermeting.push(0);

        CounterIndex {
            index: self.counters_incermeting.len() - 1,
            decrementing: false,
        }
    }

    pub fn tick(&mut self) {
        for counter in self.counters_decrementing.iter_mut() {
            counter.0 = counter.0.saturating_sub(1);

            if counter.0 == 0 {
                counter.1();
            }
        }

        for counter in self.counters_incermeting.iter_mut() {
            *counter = counter.saturating_add(1);
        }
    }

    pub fn get_counter_value(&self, index: &CounterIndex) -> u64 {
        if index.decrementing {
            self.counters_decrementing[index.index].0
        } else {
            self.counters_incermeting[index.index]
        }
    }

    // pub fn remove_counter(&mut self, index: CounterIndex) {
    //     if index.decrementing {
    //         self.counters_decrementing.remove(index.index);
    //     } else {
    //         self.counters_incermeting.remove(index.index);
    //     }
    // }
}
