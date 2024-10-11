use crate::ps2::controller::Value;
use core::ops::Not;

use super::{Response, SystemFlag};

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct ConfigurationByte(pub u8);

impl ConfigurationByte {
    pub fn get_config(&self) -> Config {
        Config {
            first_port_interrupt: self.get_first_port_interrupt(),
            second_port_interrupt: self.get_second_port_interrupt(),
            system_flag: self.get_system_flag(),
            should_be_zero: self.get_should_be_zero(),
            first_port_clock: self.get_first_port_clock(),
            second_port_clock: self.get_second_port_clock(),
            first_port_translation: self.get_first_port_translation(),
            must_be_zero: self.get_must_be_zero(),
        }
    }

    const fn get_bit(self, bit: u8) -> bool {
        self.0 & (1 << bit) != 0
    }

    fn set_bit(&mut self, bit: u8, value: bool) {
        if value {
            self.0 |= 1 << bit;
        } else {
            self.0 &= !(1 << bit);
        }
    }

    pub fn get_first_port_interrupt(&self) -> EnabledOrDisabled {
        // 1111_1111 & 0000_0001 = 0000_0001
        EnabledOrDisabled::from(self.get_bit(0))
    }

    pub fn set_first_port_interrupt(&mut self, value: EnabledOrDisabled) {
        // 1111_1110 | 0000_0001 = 1111_1111
        // 1111_1111 & 1111_1110 = 1111_1110
        self.set_bit(0, value.into());
    }

    pub fn get_second_port_interrupt(&self) -> EnabledOrDisabled {
        EnabledOrDisabled::from(self.get_bit(1))
    }

    pub fn set_second_port_interrupt(&mut self, value: EnabledOrDisabled) {
        self.set_bit(1, value.into());
    }

    pub fn get_system_flag(&self) -> SystemFlag {
        SystemFlag::from(self.get_bit(2))
    }

    pub const fn get_should_be_zero(&self) -> bool {
        self.get_bit(3)
    }

    pub fn get_first_port_clock(&self) -> EnabledOrDisabled {
        EnabledOrDisabled::from(!self.get_bit(4))
    }

    pub fn set_first_port_clock(&mut self, value: EnabledOrDisabled) {
        self.set_bit(4, bool::from(value).not());
    }

    pub fn get_second_port_clock(&self) -> EnabledOrDisabled {
        EnabledOrDisabled::from(!self.get_bit(5))
    }

    pub fn set_second_port_clock(&mut self, value: EnabledOrDisabled) {
        self.set_bit(5, bool::from(value).not());
    }

    pub fn get_first_port_translation(&self) -> EnabledOrDisabled {
        EnabledOrDisabled::from(self.get_bit(6))
    }

    pub fn set_first_port_translation(&mut self, value: EnabledOrDisabled) {
        self.set_bit(6, value.into());
    }

    pub const fn get_must_be_zero(&self) -> bool {
        self.get_bit(7)
    }
}

impl Response for ConfigurationByte {}
impl Value for ConfigurationByte {}

impl From<u8> for ConfigurationByte {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

impl From<ConfigurationByte> for u8 {
    fn from(value: ConfigurationByte) -> Self {
        value.0
    }
}

#[repr(transparent)]
pub struct UnknownPurpose(pub u8);

impl From<u8> for UnknownPurpose {
    fn from(value: u8) -> Self {
        UnknownPurpose(value)
    }
}

impl Response for UnknownPurpose {}

#[repr(u8)]
#[derive(Debug)]
pub enum PortTestResult {
    Passed = 0x00,
    ClockLineStruckLow = 0x01,
    ClockLineStruckHigh = 0x02,
    DataLineStruckLow = 0x03,
    DataLineStruckHigh = 0x04,
}

impl From<u8> for PortTestResult {
    fn from(value: u8) -> Self {
        // TODO: Make safe by using try from or smth
        unsafe { core::mem::transmute::<u8, Self>(value) }
    }
}

impl Response for PortTestResult {}

#[repr(u8)]
#[derive(Debug)]
pub enum ControllerTestResult {
    TestPassed = 0x55,
    TestFailed = 0xFC,
}

impl From<u8> for ControllerTestResult {
    fn from(value: u8) -> Self {
        // TODO: Make safe by using try from or smth
        unsafe { core::mem::transmute::<u8, Self>(value) }
    }
}

impl Response for ControllerTestResult {}

#[derive(Debug)]
pub struct Config {
    first_port_interrupt: EnabledOrDisabled,
    second_port_interrupt: EnabledOrDisabled,
    system_flag: SystemFlag,
    should_be_zero: bool,
    first_port_clock: EnabledOrDisabled,
    /// Only works if 2 PS/2 ports are supported
    second_port_clock: EnabledOrDisabled,
    first_port_translation: EnabledOrDisabled,
    must_be_zero: bool,
}

#[repr(u8)]
#[derive(Debug)]
pub enum EnabledOrDisabled {
    Disabled = 0,
    Enabled = 1,
}

impl From<bool> for EnabledOrDisabled {
    fn from(value: bool) -> Self {
        if value { Self::Enabled } else { Self::Disabled }
    }
}

impl From<EnabledOrDisabled> for bool {
    fn from(value: EnabledOrDisabled) -> Self {
        match value {
            EnabledOrDisabled::Disabled => false,
            EnabledOrDisabled::Enabled => true,
        }
    }
}
