use super::{AnyCommand, Command, CommandWithResponse, CommandWithValue};

use super::responses::ConfigurationByte;

pub struct ReadConfigurationByte;

impl Into<u8> for ReadConfigurationByte {
    fn into(self) -> u8 {
        0x20
    }
}

impl AnyCommand for ReadConfigurationByte {}
impl CommandWithResponse for ReadConfigurationByte {
    type Response = super::responses::ConfigurationByte;
}

/// TODO: Improve api to deal with offset
/// Reads byte N from internal memory, N is: (the offset + 33) & 0x1F
/// the offset can be a max off 30
pub struct ReadByteN {
    offset: u8,
}

impl Into<u8> for ReadByteN {
    fn into(self) -> u8 {
        self.offset + 0x21
    }
}

impl AnyCommand for ReadByteN {}
impl CommandWithResponse for ReadByteN {
    type Response = super::responses::UnknownPurpose;
}

/// Write next byte to byte 0 of internal ram
pub struct WriteConfigurationByte;

impl Into<u8> for WriteConfigurationByte {
    fn into(self) -> u8 {
        0x60
    }
}

impl AnyCommand for WriteConfigurationByte {}
impl CommandWithValue for WriteConfigurationByte {
    type Value = ConfigurationByte;
}

/// TODO: Improve api to deal with offset
/// Write next byte to byte N internal memory, N is: (the offset + 97) & 0x1F
/// the offset can be a max off 30
pub struct WriteByteN {
    offset: u8,
}

impl Into<u8> for WriteByteN {
    fn into(self) -> u8 {
        self.offset + 0x61
    }
}

impl AnyCommand for WriteByteN {}
impl Command for WriteByteN {}

/// Disables the second PS/2 port
/// Only works if 2 PS/2 Ports are supported
pub struct DisableSecondPort;

impl Into<u8> for DisableSecondPort {
    fn into(self) -> u8 {
        0xA7
    }
}

impl AnyCommand for DisableSecondPort {}
impl Command for DisableSecondPort {}

/// Enables the second PS/2 port
/// Only works if 2 PS/2 Ports are supported
pub struct EnableSecondPort;

impl Into<u8> for EnableSecondPort {
    fn into(self) -> u8 {
        0xA8
    }
}

impl AnyCommand for EnableSecondPort {}
impl Command for EnableSecondPort {}

/// Tests the second PS/2 port
/// Only works if 2 PS/2 Ports are supported
pub struct TestSecondPort;

impl Into<u8> for TestSecondPort {
    fn into(self) -> u8 {
        0xA9
    }
}

impl AnyCommand for TestSecondPort {}
impl CommandWithResponse for TestSecondPort {
    type Response = super::responses::PortTestResult;
}

/// Tests the controller
pub struct TestController;

impl Into<u8> for TestController {
    fn into(self) -> u8 {
        0xAA
    }
}

impl AnyCommand for TestController {}
impl CommandWithResponse for TestController {
    type Response = super::responses::ControllerTestResult;
}

/// Tests the first PS/2 port
pub struct TestFirstPort;

impl Into<u8> for TestFirstPort {
    fn into(self) -> u8 {
        0xAB
    }
}

impl AnyCommand for TestFirstPort {}
impl CommandWithResponse for TestFirstPort {
    type Response = super::responses::PortTestResult;
}

/// Reads all bytes of internal ram
pub struct DiagonsticDump;

impl Into<u8> for DiagonsticDump {
    fn into(self) -> u8 {
        0xAC
    }
}

impl AnyCommand for DiagonsticDump {}
impl CommandWithResponse for DiagonsticDump {
    type Response = super::responses::UnknownPurpose;
}

/// Disables the first PS/2 port
pub struct DisableFirstPort;

impl Into<u8> for DisableFirstPort {
    fn into(self) -> u8 {
        0xAD
    }
}

impl AnyCommand for DisableFirstPort {}
impl Command for DisableFirstPort {}

/// Enables the first PS/2 port
pub struct EnableFirstPort;

impl Into<u8> for EnableFirstPort {
    fn into(self) -> u8 {
        0xAE
    }
}

impl AnyCommand for EnableFirstPort {}
impl Command for EnableFirstPort {}

/// Reads the controller's input port
pub struct ReadControllerInputPort;

impl Into<u8> for ReadControllerInputPort {
    fn into(self) -> u8 {
        0xC0
    }
}

impl AnyCommand for ReadControllerInputPort {}
impl CommandWithResponse for ReadControllerInputPort {
    type Response = super::responses::UnknownPurpose;
}

/// Copies bits 0 to 3 of input port to status bits 4 to 7
pub struct CopyInputBits0To3ToStatusBits4To7;

impl Into<u8> for CopyInputBits0To3ToStatusBits4To7 {
    fn into(self) -> u8 {
        0xC1
    }
}

impl AnyCommand for CopyInputBits0To3ToStatusBits4To7 {}
impl Command for CopyInputBits0To3ToStatusBits4To7 {}

/// Copies bits 4 to 7 of input port to status bits 4 to 7
pub struct CopyInputBits4To7ToStatusBits4To7;

impl Into<u8> for CopyInputBits4To7ToStatusBits4To7 {
    fn into(self) -> u8 {
        0xC2
    }
}

impl AnyCommand for CopyInputBits4To7ToStatusBits4To7 {}
impl Command for CopyInputBits4To7ToStatusBits4To7 {}

/// Reads the controller's output port
pub struct ReadControllerOutputPort;

impl Into<u8> for ReadControllerOutputPort {
    fn into(self) -> u8 {
        0xD0
    }
}

impl AnyCommand for ReadControllerOutputPort {}
impl CommandWithResponse for ReadControllerOutputPort {
    type Response = super::responses::UnknownPurpose;
}

/// Output buffer must be empty first
pub struct WriteNextByteToOutputPort;

impl Into<u8> for WriteNextByteToOutputPort {
    fn into(self) -> u8 {
        0xD1
    }
}

impl AnyCommand for WriteNextByteToOutputPort {}
impl Command for WriteNextByteToOutputPort {}

/// Writes the next byte to the first PS/2 port's input buffer. This will make it look like the
/// next byte was received from the first PS/2 port.
/// Only works if 2 PS/2 Ports are supported
pub struct WriteNextByteToFirstPS2PortOutputBuffer;

impl Into<u8> for WriteNextByteToFirstPS2PortOutputBuffer {
    fn into(self) -> u8 {
        0xD2
    }
}

impl AnyCommand for WriteNextByteToFirstPS2PortOutputBuffer {}
impl Command for WriteNextByteToFirstPS2PortOutputBuffer {}

/// Writes the next byte to the second PS/2 port's input buffer. This will make it look like the
/// next byte was received from the second PS/2 port.
/// Only works if 2 PS/2 Ports are supported
pub struct WriteNextByteToSecondPS2PortOutputBuffer;

impl Into<u8> for WriteNextByteToSecondPS2PortOutputBuffer {
    fn into(self) -> u8 {
        0xD3
    }
}

impl AnyCommand for WriteNextByteToSecondPS2PortOutputBuffer {}
impl Command for WriteNextByteToSecondPS2PortOutputBuffer {}

/// Writes the next byte to the second PS/2 port's input buffer. This will send the next byte
/// to the second PS/2 port.
/// Only works if 2 PS/2 Ports are supported
pub struct WriteNextByteToSecondPS2PortInputBuffer;

impl Into<u8> for WriteNextByteToSecondPS2PortInputBuffer {
    fn into(self) -> u8 {
        0xD4
    }
}

impl AnyCommand for WriteNextByteToSecondPS2PortInputBuffer {}
impl Command for WriteNextByteToSecondPS2PortInputBuffer {}

/// Pulse output line low for 6 ms
pub struct PulseOutputLineLow;

impl Into<u8> for PulseOutputLineLow {
    fn into(self) -> u8 {
        0xF0
    }
}

impl AnyCommand for PulseOutputLineLow {}
impl Command for PulseOutputLineLow {}
