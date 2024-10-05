use responses::{ControllerTestResult, EnabledOrDisabled, PortTestResult};
use x86_64::instructions::port::{Port, PortReadOnly, PortWriteOnly};

pub mod commands;
pub mod controllers;
pub mod responses;

pub trait AnyCommand: Into<u8> {}

pub trait Command: AnyCommand {}

pub trait CommandWithResponse: AnyCommand {
    type Response: Response;
}

pub trait CommandWithValue: AnyCommand {
    type Value: Value;
}

pub trait Response: From<u8> {}
pub trait Value: Into<u8> {}

pub trait State {}

pub struct Inital;
impl State for Inital {}

pub trait InitalTrait: PS2ControllerInternal {
    type Reader: WaitingToReadTrait<u8>;
    type Writer: WaitingToWriteTrait<u8>;

    fn as_reader(self) -> Self::Reader;
    fn as_writer(self) -> Self::Writer;

    unsafe fn reset_chain<I: InitalTrait>(self) -> I;
}

#[derive(Debug)]
pub struct WaitingToRead;
impl State for WaitingToRead {}

pub trait WaitingToReadTrait<B: From<u8>> {
    type Inital: InitalTrait;
    type Ready: ReadyToReadTrait<B>;

    fn block_until_ready(self) -> Self::Ready;

    fn try_read(self) -> Result<Self::Ready, Self>
    where
        Self: Sized;

    fn is_ready(&mut self) -> bool;

    fn stop_waiting(self) -> Self::Inital;
}

pub struct ReadyToRead;
impl State for ReadyToRead {}

pub trait ReadyToReadTrait<B: From<u8> = u8> {
    type Inital: InitalTrait;

    fn read(self) -> (Self::Inital, B);
}

pub struct WaitingToWrite;
impl State for WaitingToWrite {}

pub trait WaitingToWriteTrait<B: Into<u8>> {
    type Ready: ReadyToWriteTrait<B>;

    fn block_until_ready(self) -> Self::Ready;

    fn try_read(self) -> Result<Self::Ready, Self>
    where
        Self: Sized;

    fn is_ready(&mut self) -> bool;
}

pub struct ReadyToWrite;
impl State for ReadyToWrite {}

pub trait ReadyToWriteTrait<B: Into<u8>> {
    type Inital: InitalTrait;

    fn write(self, value: B) -> Self::Inital;
}

trait PS2ControllerInternal {
    type CommandSender: CommandSenderTrait;
    type CommandSenderWithResponse: CommandSenderWithResponseTrait;
    type CommandSenderWithValue: CommandSenderWithValueTrait;

    fn into_command_sender(self) -> Self::CommandSender;
    fn into_command_sender_with_response(self) -> Self::CommandSenderWithResponse;
    fn into_command_sender_with_value(self) -> Self::CommandSenderWithValue;

    fn read_status_byte(&mut self) -> StatusByte;
}

pub struct CommandSender;
impl State for CommandSender {}

pub trait CommandSenderTrait {
    type Inital: InitalTrait;

    fn send_command<C: Command>(self, command: C) -> Self::Inital;
}

pub struct CommandSenderWithResponse;
impl State for CommandSenderWithResponse {}

pub trait CommandSenderWithResponseTrait {
    type Reader<B: Response>: WaitingToReadTrait<B>;

    /// The result reader must be read from
    fn send_command_with_response<C: CommandWithResponse>(
        self,
        command: C,
    ) -> Self::Reader<C::Response>;
}

pub struct CommandSenderWithValue;
impl State for CommandSenderWithValue {}

pub trait CommandSenderWithValueTrait {
    type Writer<B: Value>: WaitingToWriteTrait<B>;

    fn send_command_with_value<C: CommandWithValue>(self, command: C) -> Self::Writer<C::Value>;
}

pub trait PS2Controller: InitalTrait + PS2ControllerInternal {
    fn initialize(self) -> Self
    where
        Self: Sized,
    {
        let controller = self;

        let controller = controller
            .into_command_sender()
            .send_command(commands::DisableFirstPort);
        let controller = controller
            .into_command_sender()
            .send_command(commands::DisableSecondPort);

        let (controller, _) = match controller.as_reader().try_read() {
            Ok(con) => con.read(),
            Err(con) => (unsafe { con.stop_waiting().reset_chain() }, 0),
        };

        let (controller, mut config) = controller
            .into_command_sender_with_response()
            .send_command_with_response(commands::ReadConfigurationByte)
            .block_until_ready()
            .read();

        config.set_first_port_interrupt(EnabledOrDisabled::Disabled);
        config.set_first_port_translation(EnabledOrDisabled::Disabled);
        config.set_first_port_clock(EnabledOrDisabled::Enabled);

        let controller = controller
            .into_command_sender_with_value()
            .send_command_with_value(commands::WriteConfigurationByte)
            .block_until_ready()
            .write(config);

        let (controller, result) = controller
            .into_command_sender_with_response()
            .send_command_with_response(commands::TestController)
            .block_until_ready()
            .read();

        match result {
            ControllerTestResult::TestFailed => todo!("handle failed controller test"),
            _ => {}
        }

        // Resend config because a controller test sometimes resets the config
        let controller = controller
            .into_command_sender_with_value()
            .send_command_with_value(commands::WriteConfigurationByte)
            .block_until_ready()
            .write(config);

        let controller = controller
            .into_command_sender()
            .send_command(commands::EnableSecondPort);

        let (controller, mut config) = controller
            .into_command_sender_with_response()
            .send_command_with_response(commands::ReadConfigurationByte)
            .block_until_ready()
            .read();

        let is_2_ports = match config.get_second_port_clock() {
            EnabledOrDisabled::Disabled => false,
            EnabledOrDisabled::Enabled => true,
        };

        let controller = if is_2_ports {
            let controller = controller
                .into_command_sender()
                .send_command(commands::DisableSecondPort);

            config.set_second_port_interrupt(EnabledOrDisabled::Disabled);
            config.set_second_port_clock(EnabledOrDisabled::Enabled);

            let controller = controller
                .into_command_sender_with_value()
                .send_command_with_value(commands::WriteConfigurationByte)
                .block_until_ready()
                .write(config);

            // TODO: Improve method of "resetting the type"
            unsafe { controller.reset_chain::<Self>() }
        } else {
            unsafe { controller.reset_chain::<Self>() }
        };

        let (controller, test_result) = controller
            .into_command_sender_with_response()
            .send_command_with_response(commands::TestFirstPort)
            .block_until_ready()
            .read();

        let controller = match test_result {
            PortTestResult::Passed => {
                config.set_first_port_interrupt(EnabledOrDisabled::Enabled);
                let controller = controller
                    .into_command_sender()
                    .send_command(commands::EnableFirstPort);

                unsafe { controller.reset_chain::<Self>() }
            }
            _ => unsafe { controller.reset_chain::<Self>() },
        };

        let (controller, test_result) = controller
            .into_command_sender_with_response()
            .send_command_with_response(commands::TestFirstPort)
            .block_until_ready()
            .read();

        let controller = match test_result {
            PortTestResult::Passed => {
                config.set_second_port_interrupt(EnabledOrDisabled::Enabled);
                let controller = controller
                    .into_command_sender()
                    .send_command(commands::EnableSecondPort);

                unsafe { controller.reset_chain::<Self>() }
            }
            _ => unsafe { controller.reset_chain::<Self>() },
        };

        let controller = controller
            .into_command_sender_with_value()
            .send_command_with_value(commands::WriteConfigurationByte)
            .block_until_ready()
            .write(config);

        unsafe { controller.reset_chain::<Self>() }
    }
}

// pub trait PS2Controller: PS2ControllerInternal {
//     fn send_byte(&mut self, value: u8) -> Result<(), PS2ControllerSendError>;
//
//     fn read_byte(&mut self) -> Result<u8, PS2ControllerReadError>;
//
//     fn initialize(&mut self) {
//         self.send_command(commands::DisableFirstPort);
//         self.send_command(commands::DisableSecondPort);
//
//         let _ = self.read_byte();
//
//         let mut config = self.send_command_with_response(commands::ReadConfigurationByte);
//
//         config.set_first_port_interrupt(EnabledOrDisabled::Disabled);
//         config.set_first_port_translation(EnabledOrDisabled::Disabled);
//         config.set_first_port_clock(EnabledOrDisabled::Enabled);
//
//         self.send_command(commands::WriteConfigurationByte);
//         self.send_byte(config.0).unwrap();
//
//         let result = self.send_command_with_response(commands::TestController);
//
//         self.send_command(commands::WriteConfigurationByte);
//         self.send_byte(config.0).unwrap();
//
//         // Determine 2 channels
//         self.send_command(commands::EnableSecondPort);
//         config = self.send_command_with_response(commands::ReadConfigurationByte);
//
//         let is_2_ports = match config.get_second_port_clock() {
//             EnabledOrDisabled::Disabled => false,
//             EnabledOrDisabled::Enabled => true,
//         };
//
//         if is_2_ports {
//             self.send_command(commands::DisableSecondPort);
//             config.set_second_port_interrupt(EnabledOrDisabled::Disabled);
//             config.set_second_port_clock(EnabledOrDisabled::Enabled);
//
//             self.send_command(commands::WriteConfigurationByte);
//             self.send_byte(config.0).unwrap();
//         }
//
//         match self.send_command_with_response(commands::TestFirstPort) {
//             PortTestResult::Passed => {
//                 self.send_command(commands::EnableFirstPort);
//                 config.set_first_port_interrupt(EnabledOrDisabled::Enabled);
//             }
//             _ => {}
//         }
//
//         match self.send_command_with_response(commands::TestSecondPort) {
//             PortTestResult::Passed => {
//                 self.send_command(commands::EnableSecondPort);
//                 config.set_second_port_interrupt(EnabledOrDisabled::Enabled);
//             }
//             _ => {}
//         }
//
//         self.send_command(commands::WriteConfigurationByte);
//         self.send_byte(config.0).unwrap();
//     }
// }
//

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

#[derive(Debug)]
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

#[derive(Debug)]
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
#[derive(PartialEq)]
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

#[derive(Debug)]
#[repr(transparent)]
pub struct CommandRegister(PortWriteOnly<u8>);

impl CommandRegister {
    pub const fn new() -> Self {
        Self(PortWriteOnly::new(0x64))
    }

    pub fn send_command(&mut self, command: impl AnyCommand) {
        unsafe { self.0.write(command.into()) };
    }
}
