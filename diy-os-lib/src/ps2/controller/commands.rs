use super::{AnyCommand, Command, CommandWithResponse, CommandWithValue};

use super::responses::ConfigurationByte;
use diy_os_macros::{AnyCommand, Command, const_to_u8, const_value};

#[derive(const_to_u8, AnyCommand)]
#[const_value(0x20)]
pub struct ReadConfigurationByte;

impl CommandWithResponse for ReadConfigurationByte {
    type Response = super::responses::ConfigurationByte;
}

#[derive(const_to_u8, AnyCommand)]
#[const_value(value.offset + 0x21)]
// TODO: Improve api to deal with offset
/// Reads byte N from internal memory, N is: (the offset + 33) & 0x1F
/// the offset can be a max off 30
pub struct ReadByteN {
    offset: u8,
}

impl CommandWithResponse for ReadByteN {
    type Response = super::responses::UnknownPurpose;
}

/// Write next byte to byte 0 of internal ram
#[derive(const_to_u8, AnyCommand)]
#[const_value(0x60)]
pub struct WriteConfigurationByte;

impl CommandWithValue for WriteConfigurationByte {
    type Value = ConfigurationByte;
}

// TODO: Improve api to deal with offset
/// Write next byte to byte N internal memory, N is: (the offset + 97) & 0x1F
/// the offset can be a max off 30
#[derive(const_to_u8, AnyCommand, Command)]
#[const_value(value.offset + 0x61)]
pub struct WriteByteN {
    offset: u8,
}

/// Disables the second PS/2 port
/// Only works if 2 PS/2 Ports are supported
#[derive(const_to_u8, AnyCommand, Command)]
#[const_value(0xA7)]
pub struct DisableSecondPort;

/// Enables the second PS/2 port
/// Only works if 2 PS/2 Ports are supported
#[derive(const_to_u8, AnyCommand, Command)]
#[const_value(0xA8)]
pub struct EnableSecondPort;

/// Tests the second PS/2 port
/// Only works if 2 PS/2 Ports are supported
#[derive(const_to_u8, AnyCommand)]
#[const_value(0xA9)]
pub struct TestSecondPort;

impl CommandWithResponse for TestSecondPort {
    type Response = super::responses::PortTestResult;
}

/// Tests the controller
#[derive(const_to_u8, AnyCommand)]
#[const_value(0xAA)]
pub struct TestController;

impl CommandWithResponse for TestController {
    type Response = super::responses::ControllerTestResult;
}

/// Tests the first PS/2 port
#[derive(const_to_u8, AnyCommand)]
#[const_value(0xAB)]
pub struct TestFirstPort;

impl CommandWithResponse for TestFirstPort {
    type Response = super::responses::PortTestResult;
}

/// Reads all bytes of internal ram
#[derive(const_to_u8, AnyCommand)]
#[const_value(0xAC)]
pub struct DiagonsticDump;

impl CommandWithResponse for DiagonsticDump {
    type Response = super::responses::UnknownPurpose;
}

/// Disables the first PS/2 port
#[derive(const_to_u8, AnyCommand, Command)]
#[const_value(0xAD)]
pub struct DisableFirstPort;

/// Enables the first PS/2 port
#[derive(const_to_u8, AnyCommand, Command)]
#[const_value(0xAE)]
pub struct EnableFirstPort;

/// Reads the controller's input port
#[derive(const_to_u8, AnyCommand)]
#[const_value(0xC0)]
pub struct ReadControllerInputPort;

impl CommandWithResponse for ReadControllerInputPort {
    type Response = super::responses::UnknownPurpose;
}

/// Copies bits 0 to 3 of input port to status bits 4 to 7
#[derive(const_to_u8, AnyCommand, Command)]
#[const_value(0xC1)]
pub struct CopyInputBits0To3ToStatusBits4To7;

/// Copies bits 4 to 7 of input port to status bits 4 to 7
#[derive(const_to_u8, AnyCommand, Command)]
#[const_value(0xC2)]
pub struct CopyInputBits4To7ToStatusBits4To7;

/// Reads the controller's output port
#[derive(const_to_u8, AnyCommand)]
#[const_value(0xD0)]
pub struct ReadControllerOutputPort;

impl CommandWithResponse for ReadControllerOutputPort {
    type Response = super::responses::UnknownPurpose;
}

/// Output buffer must be empty first
#[derive(const_to_u8, AnyCommand, Command)]
#[const_value(0xD1)]
pub struct WriteNextByteToOutputPort;

/// Writes the next byte to the first PS/2 port's input buffer. This will make it look like the
/// next byte was received from the first PS/2 port.
/// Only works if 2 PS/2 Ports are supported
#[derive(const_to_u8, AnyCommand, Command)]
#[const_value(0xD2)]
pub struct WriteNextByteToFirstPS2PortOutputBuffer;

/// Writes the next byte to the second PS/2 port's input buffer. This will make it look like the
/// next byte was received from the second PS/2 port.
/// Only works if 2 PS/2 Ports are supported
#[derive(const_to_u8, AnyCommand, Command)]
#[const_value(0xD3)]
pub struct WriteNextByteToSecondPS2PortOutputBuffer;

/// Writes the next byte to the second PS/2 port's input buffer. This will send the next byte
/// to the second PS/2 port.
/// Only works if 2 PS/2 Ports are supported
#[derive(const_to_u8, AnyCommand, Command)]
#[const_value(0xD4)]
pub struct WriteNextByteToSecondPS2PortInputBuffer;

/// Pulse output line low for 6 ms
#[derive(const_to_u8, AnyCommand, Command)]
#[const_value(0xF0)]
pub struct PulseOutputLineLow;
