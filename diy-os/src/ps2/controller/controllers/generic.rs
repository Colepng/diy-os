use crate::ps2::controller::{
    BufferStatus, CommandRegister, CommandSender, CommandSenderTrait, CommandSenderWithResponse,
    CommandSenderWithResponseTrait, DataPort, Inital, InitalTrait, PS2Controller,
    PS2ControllerInternal, ReadyToRead, ReadyToReadTrait, ReadyToWrite, ReadyToWriteTrait,
    Response, State, StatusRegister, WaitingToRead, WaitingToReadTrait, WaitingToWrite,
    WaitingToWriteTrait,
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
}

impl PS2Controller for GenericPS2Controller<Inital> {}

// impl PS2ControllerInternal for GenericPS2Controller {
//     fn send_command<C: Command>(&mut self, command: C) {
//         self.command_register.send_command(command);
//     }
//
//     fn send_command_with_response<C: CommandWithResponse>(&mut self, command: C) -> C::Response {
//         self.command_register.send_command_with_response(command);
//
//         C::Response::from(self.data_port.read())
//     }
//
//     fn read_status_byte(&mut self) -> StatusByte {
//         self.status_register.read()
//     }
// }
//
// impl PS2Controller for GenericPS2Controller {
//     fn send_byte(&mut self, value: u8) -> Result<(), PS2ControllerSendError> {
//         match self.status_register.read().get_input_buffer_status() {
//             BufferStatus::Empty => {
//                 self.data_port.write(value);
//                 Ok(())
//             }
//             BufferStatus::Full => Err(PS2ControllerSendError::InputBufferFull),
//         }
//     }
//
//     fn read_byte(&mut self) -> Result<u8, PS2ControllerReadError> {
//         match self.status_register.read().get_output_buffer_status() {
//             BufferStatus::Full => Ok(self.data_port.read()),
//             BufferStatus::Empty => Err(PS2ControllerReadError::OutputBufferEmpty),
//         }
//     }
// }

impl InitalTrait for GenericPS2Controller<Inital> {
    type Reader = GenericPS2Controller<WaitingToRead>;
    type Writer = GenericPS2Controller<WaitingToWrite>;

    fn as_reader(self) -> Self::Reader {
        GenericPS2Controller::<WaitingToRead> {
            data_port: self.data_port,
            status_register: self.status_register,
            command_register: self.command_register,
            _phantom_data: PhantomData,
        }
    }

    fn as_writer(self) -> Self::Writer {
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

impl WaitingToWriteTrait for GenericPS2Controller<WaitingToWrite> {
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

impl ReadyToWriteTrait for GenericPS2Controller<ReadyToWrite> {
    type Inital = GenericPS2Controller<Inital>;

    fn write(mut self, value: u8) -> Self::Inital {
        self.data_port.write(value);

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
        self.command_register.send_command_with_response(command);

        GenericPS2Controller::<WaitingToRead> {
            data_port: self.data_port,
            status_register: self.status_register,
            command_register: self.command_register,
            _phantom_data: PhantomData,
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
