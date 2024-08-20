use crate::ps2::controller::{
    BufferStatus, Command, CommandWithResponse, PS2Controller, PS2ControllerInternal,
    PS2ControllerReadError, PS2ControllerSendError, StatusByte,
};
use crate::ps2::controller::{CommandRegister, DataPort, StatusRegister};

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

impl PS2ControllerInternal for GenericPS2Controller {
    fn send_command<C: Command>(&mut self, command: C) {
        self.command_register.send_command(command);
    }

    fn send_command_with_response<C: CommandWithResponse>(&mut self, command: C) -> C::Response {
        self.command_register.send_command_with_response(command);

        C::Response::from(self.data_port.read())
    }

    fn read_status_byte(&mut self) -> StatusByte {
        self.status_register.read()
    }
}

impl PS2Controller for GenericPS2Controller {
    fn send_byte(&mut self, value: u8) -> Result<(), PS2ControllerSendError> {
        match self.status_register.read().get_input_buffer_status() {
            BufferStatus::Empty => {
                self.data_port.write(value);
                Ok(())
            }
            BufferStatus::Full => Err(PS2ControllerSendError::InputBufferFull),
        }
    }

    fn read_byte(&mut self) -> Result<u8, PS2ControllerReadError> {
        match self.status_register.read().get_output_buffer_status() {
            BufferStatus::Full => Ok(self.data_port.read()),
            BufferStatus::Empty => Err(PS2ControllerReadError::OutputBufferEmpty),
        }
    }
}
mod model {

    pub trait State {}

    pub struct Reading;
    impl State for Reading {}

    pub struct PollingOutput;
    impl State for PollingOutput {}

    pub trait ReadingTrait {
        type Next: PollingOutputTrait;

        fn poll(self) -> Self::Next;
    }

    pub trait PollingOutputTrait {
        type Next: ReadingTrait;

        fn read(self) -> Self::Next;
    }

    pub struct Controller<S: State> {
        _phantom: core::marker::PhantomData<S>,
    }

    impl ReadingTrait for Controller<Reading> {
        type Next = Controller<PollingOutput>;

        fn poll(self) -> Self::Next {
            Controller::<PollingOutput> {
                _phantom: core::marker::PhantomData,
            }
        }
    }

    impl PollingOutputTrait for Controller<PollingOutput> {
        type Next = Controller<Reading>;

        fn read(self) -> Self::Next {
            Controller::<Reading> {
                _phantom: core::marker::PhantomData,
            }
        }
    }

    pub fn test<T: ReadingTrait>(controller: T) {
        let new_state = controller.poll();
        let a = new_state.read();
        let new_state = a.poll();
    }
}
