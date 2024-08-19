use x86_64::instructions::port::{Port, PortReadOnly, PortWriteOnly};

pub mod gernaric;

pub trait PS2Controller {
    fn send_byte(&mut self, value: u8) -> Result<(), PS2ControllerSendError>;

    fn send_command<C: Command>(&mut self, command: C);

    fn send_command_with_response<C: CommandWithResponse>(&mut self, command: C) -> C::Response;

    fn read_byte(&mut self) -> Result<u8, PS2ControllerReadError>;

    fn read_status_byte(&mut self) -> StatusByte;

    fn initialize(&mut self) {
        self.send_command(DisableFirstPort);
        self.send_command(DisableSecondPort);

        let _ = self.read_byte();
    }
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
    pub fn new() -> Self {
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
    pub fn new() -> Self {
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

    pub fn get_chipset_specifc_1(&self) -> bool {
        (self.0 & (1 << 4)) != 0
    }
    pub fn get_chipset_specifc_2(&self) -> bool {
        (self.0 & (1 << 5)) != 0
    }
    pub fn get_timeout_error(&self) -> bool {
        (self.0 & (1 << 6)) != 0
    }
    pub fn get_parity_error(&self) -> bool {
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
    pub fn new() -> Self {
        Self(PortWriteOnly::new(0x64))
    }

    pub fn send_command(&mut self, command: impl Command) -> Option<u8> {
        unsafe { self.0.write(command.into()) };

        None
    }
}

pub trait Command: Into<u8> {}

pub trait CommandWithResponse: Into<u8> {
    type Response: Response;
}

pub trait Response: From<u8> {}

pub struct ReadByte0;

impl Into<u8> for ReadByte0 {
    fn into(self) -> u8 {
        0x20
    }
}

impl CommandWithResponse for ReadByte0 {
    type Response = ConfigurationByte;
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

impl CommandWithResponse for ReadByteN {
    type Response = UnknownPurpose;
}

/// Write next byte to byte 0 of internal ram
pub struct WriteByte0;

impl Into<u8> for WriteByte0 {
    fn into(self) -> u8 {
        0x60
    }
}

impl Command for WriteByte0 {}

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

impl Command for WriteByteN {}

/// Disables the second PS/2 port
/// Only works if 2 PS/2 Ports are supported
pub struct DisableSecondPort;

impl Into<u8> for DisableSecondPort {
    fn into(self) -> u8 {
        0xA7
    }
}

impl Command for DisableSecondPort {}

/// Enables the second PS/2 port
/// Only works if 2 PS/2 Ports are supported
pub struct EnableSecondPort;

impl Into<u8> for EnableSecondPort {
    fn into(self) -> u8 {
        0xA8
    }
}

impl Command for EnableSecondPort {}

/// Tests the second PS/2 port
/// Only works if 2 PS/2 Ports are supported
pub struct TestSecondPort;

impl Into<u8> for TestSecondPort {
    fn into(self) -> u8 {
        0xA9
    }
}

impl CommandWithResponse for TestSecondPort {
    type Response = PortTestResult;
}

/// Tests the controller
pub struct TestController;

impl Into<u8> for TestController {
    fn into(self) -> u8 {
        0xAA
    }
}

impl CommandWithResponse for TestController {
    type Response = ControllerTestResult;
}

/// Tests the first PS/2 port
pub struct TestFirstPort;

impl Into<u8> for TestFirstPort {
    fn into(self) -> u8 {
        0xAB
    }
}

impl CommandWithResponse for TestFirstPort {
    type Response = PortTestResult;
}

/// Reads all bytes of internal ram
pub struct DiagonsticDump;

impl Into<u8> for DiagonsticDump {
    fn into(self) -> u8 {
        0xAC
    }
}

impl CommandWithResponse for DiagonsticDump {
    type Response = UnknownPurpose;
}

/// Disables the first PS/2 port
pub struct DisableFirstPort;

impl Into<u8> for DisableFirstPort {
    fn into(self) -> u8 {
        0xAD
    }
}

impl Command for DisableFirstPort {}

/// Enables the first PS/2 port
pub struct EnableFirstPort;

impl Into<u8> for EnableFirstPort {
    fn into(self) -> u8 {
        0xAE
    }
}

impl Command for EnableFirstPort {}

/// Reads the controller's input port
pub struct ReadControllerInputPort;

impl Into<u8> for ReadControllerInputPort {
    fn into(self) -> u8 {
        0xC0
    }
}

impl CommandWithResponse for ReadControllerInputPort {
    type Response = UnknownPurpose;
}

/// Copies bits 0 to 3 of input port to status bits 4 to 7
pub struct CopyInputBits0To3ToStatusBits4To7;

impl Into<u8> for CopyInputBits0To3ToStatusBits4To7 {
    fn into(self) -> u8 {
        0xC1
    }
}

impl Command for CopyInputBits0To3ToStatusBits4To7 {}

/// Copies bits 4 to 7 of input port to status bits 4 to 7
pub struct CopyInputBits4To7ToStatusBits4To7;

impl Into<u8> for CopyInputBits4To7ToStatusBits4To7 {
    fn into(self) -> u8 {
        0xC2
    }
}

impl Command for CopyInputBits4To7ToStatusBits4To7 {}

/// Reads the controller's output port
pub struct ReadControllerOutputPort;

impl Into<u8> for ReadControllerOutputPort {
    fn into(self) -> u8 {
        0xD0
    }
}

impl CommandWithResponse for ReadControllerOutputPort {
    type Response = UnknownPurpose;
}

/// Output buffer must be empty first
pub struct WriteNextByteToOutputPort;

impl Into<u8> for WriteNextByteToOutputPort {
    fn into(self) -> u8 {
        0xD1
    }
}

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

impl Command for WriteNextByteToSecondPS2PortInputBuffer {}

/// Pulse output line low for 6 ms
pub struct PulseOutputLineLow;

impl Into<u8> for PulseOutputLineLow {
    fn into(self) -> u8 {
        0xF0
    }
}

impl Command for PulseOutputLineLow {}

#[repr(transparent)]
pub struct UnknownPurpose(pub u8);

impl From<u8> for UnknownPurpose {
    fn from(value: u8) -> Self {
        UnknownPurpose(value)
    }
}

impl Response for UnknownPurpose {}

#[repr(u8)]
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

#[repr(transparent)]
pub struct ConfigurationByte(u8);

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

    pub fn get_first_port_interrupt(&self) -> EnabledOrDisabled {
        ((self.0 & (1 << 0)) != 0).into()
    }

    pub fn get_second_port_interrupt(&self) -> EnabledOrDisabled {
        ((self.0 & (1 << 1)) != 0).into()
    }

    pub fn get_system_flag(&self) -> SystemFlag {
        ((self.0 & (1 << 2)) != 0).into()
    }

    pub fn get_should_be_zero(&self) -> bool {
        (self.0 & (1 << 3)) != 0
    }

    pub fn get_first_port_clock(&self) -> EnabledOrDisabled {
        (!((self.0 & (1 << 4)) != 0)).into()
    }

    pub fn get_second_port_clock(&self) -> EnabledOrDisabled {
        (!((self.0 & (1 << 5)) != 0)).into()
    }

    pub fn get_first_port_translation(&self) -> EnabledOrDisabled {
        ((self.0 & (1 << 6)) != 0).into()
    }
    pub fn get_must_be_zero(&self) -> bool {
        (self.0 & (1 << 7)) != 0
    }
}

impl Response for ConfigurationByte {}

impl From<u8> for ConfigurationByte {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

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
pub enum EnabledOrDisabled {
    Disabled = 0,
    Enabled = 1,
}

impl From<bool> for EnabledOrDisabled {
    fn from(value: bool) -> Self {
        // Safety: Safe to transmute between bool and EnableOrDisabled
        // since a bool must be a 0 or 1
        unsafe { core::mem::transmute::<bool, EnabledOrDisabled>(value) }
    }
}
