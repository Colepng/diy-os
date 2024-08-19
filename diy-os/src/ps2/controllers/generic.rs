use crate::ps2::controllers::DataPort;
use crate::ps2::controllers::StatusRegister;

use super::BufferStatus;
use super::CommandRegister;
use super::PS2Controller;

pub struct GenericPS2Controller {
    data_port: DataPort,
    status_register: StatusRegister,
    command_register: CommandRegister,
}

impl GenericPS2Controller {
    pub const fn new() -> Self {
        Self {
            data_port: DataPort::new(),
            status_register: StatusRegister::new(),
            command_register: CommandRegister::new(),
        }
    }
}

impl PS2Controller for GenericPS2Controller {
    fn send_byte(&mut self, value: u8) -> Result<(), super::PS2ControllerSendError> {
        match self.status_register.read().get_input_buffer_status() {
            BufferStatus::Empty => {
                self.data_port.write(value);
                Ok(())
            }
            BufferStatus::Full => Err(super::PS2ControllerSendError::InputBufferFull),
        }
    }

    fn read_byte(&mut self) -> Result<u8, super::PS2ControllerReadError> {
        match self.status_register.read().get_output_buffer_status() {
            BufferStatus::Full => Ok(self.data_port.read()),
            BufferStatus::Empty => Err(super::PS2ControllerReadError::OutputBufferEmpty),
        }
    }

    fn read_status_byte(&mut self) -> super::StatusByte {
        self.status_register.read()
    }

    fn send_command<C: super::Command>(&mut self, command: C) {
        self.command_register.send_command(command);
    }

    fn send_command_with_response<C: super::CommandWithResponse>(
        &mut self,
        command: C,
    ) -> C::Response {
        self.command_register.send_command_with_response(command);

        C::Response::from(self.data_port.read())
    }
}
