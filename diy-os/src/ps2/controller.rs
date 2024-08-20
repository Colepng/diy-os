use crate::ps2::controller::responses::EnabledOrDisabled;
use crate::ps2::controller::responses::PortTestResult;
use x86_64::instructions::port::{Port, PortReadOnly, PortWriteOnly};

pub mod commands;
pub mod controllers;
pub mod responses;

pub trait Command: Into<u8> {}

pub trait CommandWithResponse: Into<u8> {
    type Response: Response;
}

pub trait Response: From<u8> {}

pub struct State;

pub trait PS2Controller: PS2ControllerInternal {
    fn send_byte(&mut self, value: u8) -> Result<(), PS2ControllerSendError>;

    fn read_byte(&mut self) -> Result<u8, PS2ControllerReadError>;

    fn initialize(&mut self) {
        self.send_command(commands::DisableFirstPort);
        self.send_command(commands::DisableSecondPort);

        let _ = self.read_byte();

        let mut config = self.send_command_with_response(commands::ReadConfigurationByte);

        config.set_first_port_interrupt(EnabledOrDisabled::Disabled);
        config.set_first_port_translation(EnabledOrDisabled::Disabled);
        config.set_first_port_clock(EnabledOrDisabled::Enabled);

        self.send_command(commands::WriteConfigurationByte);
        self.send_byte(config.0).unwrap();

        let result = self.send_command_with_response(commands::TestController);

        self.send_command(commands::WriteConfigurationByte);
        self.send_byte(config.0).unwrap();

        // Determine 2 channels
        self.send_command(commands::EnableSecondPort);
        config = self.send_command_with_response(commands::ReadConfigurationByte);

        let is_2_ports = match config.get_second_port_clock() {
            EnabledOrDisabled::Disabled => false,
            EnabledOrDisabled::Enabled => true,
        };

        if is_2_ports {
            self.send_command(commands::DisableSecondPort);
            config.set_second_port_interrupt(EnabledOrDisabled::Disabled);
            config.set_second_port_clock(EnabledOrDisabled::Enabled);

            self.send_command(commands::WriteConfigurationByte);
            self.send_byte(config.0).unwrap();
        }

        match self.send_command_with_response(commands::TestFirstPort) {
            PortTestResult::Passed => {
                self.send_command(commands::EnableFirstPort);
                config.set_first_port_interrupt(EnabledOrDisabled::Enabled);
            }
            _ => {}
        }

        match self.send_command_with_response(commands::TestSecondPort) {
            PortTestResult::Passed => {
                self.send_command(commands::EnableSecondPort);
                config.set_second_port_interrupt(EnabledOrDisabled::Enabled);
            }
            _ => {}
        }

        self.send_command(commands::WriteConfigurationByte);
        self.send_byte(config.0).unwrap();
    }
}

trait PS2ControllerInternal {
    fn send_command<C: Command>(&mut self, command: C);

    fn send_command_with_response<C: CommandWithResponse>(&mut self, command: C) -> C::Response;

    fn read_status_byte(&mut self) -> StatusByte;
}

#[derive(thiserror::Error, Debug)]
pub enum PS2ControllerReadError {
    #[error("Output buffer was empty")]
    OutputBufferEmpty,
}

#[derive(thiserror::Error, Debug)]
pub enum PS2ControllerSendError {
    #[error("Input buffer was full")]
    InputBufferFull,
}

#[repr(transparent)]
pub struct DataPort(Port<u8>);

impl DataPort {
    pub const fn new() -> Self {
        Self(Port::new(0x60))
    }

    pub fn read(&mut self) -> u8 {
        unsafe { self.0.read() }
    }

    pub fn write(&mut self, value: u8) {
        unsafe {
            self.0.write(value);
        }
    }
}

#[repr(transparent)]
pub struct StatusRegister(PortReadOnly<u8>);

impl StatusRegister {
    pub const fn new() -> Self {
        Self(PortReadOnly::new(0x64))
    }

    pub fn read(&mut self) -> StatusByte {
        StatusByte(unsafe { self.0.read() })
    }
}

#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct StatusByte(u8);

impl StatusByte {
    pub fn get_status(&self) -> Status {
        Status {
            output_buffer_status: self.get_output_buffer_status(),
            input_buffer_status: self.get_input_buffer_status(),
            system_flag: self.get_system_flag(),
            command_or_data: self.get_command_or_data(),
            chipset_specifc_1: self.get_chipset_specifc_1(),
            chipset_specifc_2: self.get_chipset_specifc_2(),
            timeout_error: self.get_timeout_error(),
            parity_error: self.get_parity_error(),
        }
    }

    pub fn get_output_buffer_status(&self) -> BufferStatus {
        ((self.0 & (1 << 0)) != 0).into()
    }

    pub fn get_input_buffer_status(&self) -> BufferStatus {
        ((self.0 & (1 << 1)) != 0).into()
    }

    pub fn get_system_flag(&self) -> SystemFlag {
        ((self.0 & (1 << 2)) != 0).into()
    }

    pub fn get_command_or_data(&self) -> CommandOrData {
        ((self.0 & (1 << 3)) != 0).into()
    }

    pub const fn get_chipset_specifc_1(&self) -> bool {
        (self.0 & (1 << 4)) != 0
    }
    pub const fn get_chipset_specifc_2(&self) -> bool {
        (self.0 & (1 << 5)) != 0
    }
    pub const fn get_timeout_error(&self) -> bool {
        (self.0 & (1 << 6)) != 0
    }
    pub const fn get_parity_error(&self) -> bool {
        (self.0 & (1 << 7)) != 0
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct Status {
    output_buffer_status: BufferStatus,
    input_buffer_status: BufferStatus,
    system_flag: SystemFlag,
    command_or_data: CommandOrData,
    chipset_specifc_1: bool,
    chipset_specifc_2: bool,
    timeout_error: bool,
    parity_error: bool,
}

#[derive(Debug)]
#[repr(u8)]
pub enum BufferStatus {
    Empty = 0,
    Full = 1,
}

impl From<bool> for BufferStatus {
    fn from(value: bool) -> Self {
        // Safety: Safe to transmute between bool and BufferStatus
        // since a bool must be a 0 or 1
        unsafe { core::mem::transmute::<bool, BufferStatus>(value) }
    }
}

#[derive(Debug)]
#[repr(u8)]
pub enum SystemFlag {
    SystemFalledPOST = 0,
    SystemPassedPOST = 1,
}

impl From<bool> for SystemFlag {
    fn from(value: bool) -> Self {
        // Safety: Safe to transmute between bool and SystemFlag
        // since a bool must be a 0 or 1
        unsafe { core::mem::transmute::<bool, SystemFlag>(value) }
    }
}

#[derive(Debug)]
#[repr(u8)]
pub enum CommandOrData {
    Data = 0,
    Command = 1,
}

impl From<bool> for CommandOrData {
    fn from(value: bool) -> Self {
        // Safety: Safe to transmute between bool and CommandOrData
        // since a bool must be a 0 or 1
        unsafe { core::mem::transmute::<bool, CommandOrData>(value) }
    }
}

#[repr(transparent)]
pub struct CommandRegister(PortWriteOnly<u8>);

impl CommandRegister {
    pub const fn new() -> Self {
        Self(PortWriteOnly::new(0x64))
    }

    pub fn send_command(&mut self, command: impl Command) {
        unsafe { self.0.write(command.into()) };
    }

    pub fn send_command_with_response(&mut self, command: impl CommandWithResponse) {
        unsafe { self.0.write(command.into()) }
    }
}
