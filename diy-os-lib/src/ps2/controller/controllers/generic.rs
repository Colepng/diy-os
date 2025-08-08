use crate::ps2::controller::{
    BufferStatus, CommandRegister, CommandSender, CommandSenderTrait, CommandSenderWithResponse,
    CommandSenderWithResponseTrait, CommandSenderWithValue, CommandSenderWithValueTrait, DataPort,
    Inital, InitalTrait, PS2Controller, PS2ControllerInternal, ReadyToRead, ReadyToReadTrait,
    ReadyToWrite, ReadyToWriteTrait, Response, State, StatusRegister, WaitingToRead,
    WaitingToReadTrait, WaitingToWrite, WaitingToWriteTrait,
};
use core::marker::PhantomData;

#[derive(Debug)]
pub struct GenericPS2Controller<S: State> {
    data_port: DataPort,
    status_register: StatusRegister,
    command_register: CommandRegister,
    _phantom_data: PhantomData<S>,
}

impl GenericPS2Controller<Inital> {
    pub const fn new() -> Self {
        Self {
            data_port: DataPort::new(),
            status_register: StatusRegister::new(),
            command_register: CommandRegister::new(),
            _phantom_data: PhantomData::<Inital>,
        }
    }
}

impl PS2ControllerInternal for GenericPS2Controller<Inital> {
    type CommandSender = GenericPS2Controller<CommandSender>;
    type CommandSenderWithResponse = GenericPS2Controller<CommandSenderWithResponse>;
    type CommandSenderWithValue = GenericPS2Controller<CommandSenderWithValue>;

    fn into_command_sender(self) -> Self::CommandSender {
        GenericPS2Controller::<CommandSender> {
            data_port: self.data_port,
            status_register: self.status_register,
            command_register: self.command_register,
            _phantom_data: PhantomData,
        }
    }

    fn read_status_byte(&mut self) -> crate::ps2::controller::StatusByte {
        self.status_register.read()
    }

    fn into_command_sender_with_response(self) -> Self::CommandSenderWithResponse {
        GenericPS2Controller::<CommandSenderWithResponse> {
            data_port: self.data_port,
            status_register: self.status_register,
            command_register: self.command_register,
            _phantom_data: PhantomData,
        }
    }

    fn into_command_sender_with_value(self) -> Self::CommandSenderWithValue {
        GenericPS2Controller::<CommandSenderWithValue> {
            data_port: self.data_port,
            status_register: self.status_register,
            command_register: self.command_register,
            _phantom_data: PhantomData,
        }
    }
}

impl PS2Controller for GenericPS2Controller<Inital> {}

impl InitalTrait for GenericPS2Controller<Inital> {
    type Reader = GenericPS2Controller<WaitingToRead>;
    type Writer = GenericPS2Controller<WaitingToWrite>;

    fn into_reader(self) -> Self::Reader {
        GenericPS2Controller::<WaitingToRead> {
            data_port: self.data_port,
            status_register: self.status_register,
            command_register: self.command_register,
            _phantom_data: PhantomData,
        }
    }

    fn into_writer(self) -> Self::Writer {
        GenericPS2Controller::<WaitingToWrite> {
            data_port: self.data_port,
            status_register: self.status_register,
            command_register: self.command_register,
            _phantom_data: PhantomData,
        }
    }

    unsafe fn reset_chain<I: InitalTrait>(self) -> I {
        unsafe { core::mem::transmute_copy::<Self, I>(&self) }
    }
}

impl<B: From<u8>> WaitingToReadTrait<B> for GenericPS2Controller<WaitingToRead> {
    type Inital = GenericPS2Controller<Inital>;
    type Ready = GenericPS2Controller<ReadyToRead>;

    fn block_until_ready(mut self) -> Self::Ready {
        while self.status_register.read().get_output_buffer_status() != BufferStatus::Full {}

        GenericPS2Controller::<ReadyToRead> {
            data_port: self.data_port,
            status_register: self.status_register,
            command_register: self.command_register,
            _phantom_data: PhantomData,
        }
    }

    fn try_read(mut self) -> Result<Self::Ready, Self>
    where
        Self: Sized,
    {
        match self.status_register.read().get_output_buffer_status() {
            BufferStatus::Empty => Err(self),
            BufferStatus::Full => Ok(GenericPS2Controller::<ReadyToRead> {
                data_port: self.data_port,
                status_register: self.status_register,
                command_register: self.command_register,
                _phantom_data: PhantomData,
            }),
        }
    }

    fn is_ready(&mut self) -> bool {
        self.status_register.read().get_output_buffer_status() == BufferStatus::Full
    }

    fn stop_waiting(self) -> Self::Inital {
        GenericPS2Controller::<Inital> {
            data_port: self.data_port,
            status_register: self.status_register,
            command_register: self.command_register,
            _phantom_data: PhantomData,
        }
    }
}

impl<B: From<u8>> ReadyToReadTrait<B> for GenericPS2Controller<ReadyToRead> {
    type Inital = GenericPS2Controller<Inital>;

    fn read(mut self) -> (Self::Inital, B) {
        let result = self.data_port.read();

        (
            GenericPS2Controller::<Inital> {
                data_port: self.data_port,
                status_register: self.status_register,
                command_register: self.command_register,
                _phantom_data: PhantomData,
            },
            result.into(),
        )
    }
}

impl<B: Into<u8>> WaitingToWriteTrait<B> for GenericPS2Controller<WaitingToWrite> {
    type Ready = GenericPS2Controller<ReadyToWrite>;

    fn block_until_ready(mut self) -> Self::Ready {
        while self.status_register.read().get_input_buffer_status() == BufferStatus::Full {}

        GenericPS2Controller::<ReadyToWrite> {
            data_port: self.data_port,
            status_register: self.status_register,
            command_register: self.command_register,
            _phantom_data: PhantomData,
        }
    }

    fn try_read(mut self) -> Result<Self::Ready, Self>
    where
        Self: Sized,
    {
        match self.status_register.read().get_input_buffer_status() {
            BufferStatus::Empty => Ok(GenericPS2Controller::<ReadyToWrite> {
                data_port: self.data_port,
                status_register: self.status_register,
                command_register: self.command_register,
                _phantom_data: PhantomData,
            }),
            BufferStatus::Full => Err(self),
        }
    }

    fn is_ready(&mut self) -> bool {
        self.status_register.read().get_input_buffer_status() == BufferStatus::Empty
    }
}

impl<B: Into<u8>> ReadyToWriteTrait<B> for GenericPS2Controller<ReadyToWrite> {
    type Inital = GenericPS2Controller<Inital>;

    fn write(mut self, value: B) -> Self::Inital {
        self.data_port.write(value.into());

        GenericPS2Controller::<Inital> {
            data_port: self.data_port,
            status_register: self.status_register,
            command_register: self.command_register,
            _phantom_data: PhantomData,
        }
    }
}

impl CommandSenderTrait for GenericPS2Controller<CommandSender> {
    type Inital = GenericPS2Controller<Inital>;

    fn send_command<C: crate::ps2::controller::Command>(mut self, command: C) -> Self::Inital {
        self.command_register.send_command(command);

        GenericPS2Controller::<Inital> {
            data_port: self.data_port,
            status_register: self.status_register,
            command_register: self.command_register,
            _phantom_data: PhantomData,
        }
    }
}

impl CommandSenderWithResponseTrait for GenericPS2Controller<CommandSenderWithResponse> {
    type Reader<B: Response> = GenericPS2Controller<WaitingToRead>;

    fn send_command_with_response<C: crate::ps2::controller::CommandWithResponse>(
        mut self,
        command: C,
    ) -> Self::Reader<C::Response> {
        self.command_register.send_command(command);

        GenericPS2Controller::<WaitingToRead> {
            data_port: self.data_port,
            status_register: self.status_register,
            command_register: self.command_register,
            _phantom_data: PhantomData,
        }
    }
}

impl CommandSenderWithValueTrait for GenericPS2Controller<CommandSenderWithValue> {
    type Writer<B: crate::ps2::controller::Value> = GenericPS2Controller<WaitingToWrite>;

    fn send_command_with_value<C: crate::ps2::controller::CommandWithValue>(
        mut self,
        command: C,
    ) -> Self::Writer<C::Value> {
        self.command_register.send_command(command);

        GenericPS2Controller::<WaitingToWrite> {
            data_port: self.data_port,
            status_register: self.status_register,
            command_register: self.command_register,
            _phantom_data: PhantomData,
        }
    }
}
