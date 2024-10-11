use alloc::vec::Vec;

use crate::{
    collections::queues::LinkedQueue,
    println,
    ps2::{
        CONTROLLER,
        controller::{InitalTrait, ReadyToWriteTrait, WaitingToWriteTrait},
    },
    spinlock::Spinlock,
};

use super::PS2Device;

pub static SCANCODE_BUFFER: Spinlock<Vec<ScanCode>> = Spinlock::new(Vec::new());

pub enum State {
    Idle,
    ReceivedScanCode(ScanCode),
    ReceivedReleasedCode(ScanCodeBuilder),
    ReceivedExtenededCode(ScanCodeBuilder),
    CommandReady,
    WaitingForResponse,
    GotResponse(u8),
}

#[derive(Debug, Clone, Copy)]
pub struct ScanCodeBuilder {
    scan_code: Option<u8>,
    is_released: Option<bool>,
    is_extended: Option<bool>,
}

impl ScanCodeBuilder {
    pub const fn new() -> Self {
        Self {
            scan_code: None,
            is_released: None,
            is_extended: None,
        }
    }

    pub const fn set_code(self, code: u8) -> Self {
        Self {
            scan_code: Some(code),
            is_released: self.is_released,
            is_extended: self.is_extended,
        }
    }

    pub const fn set_released(self, released: bool) -> Self {
        Self {
            scan_code: self.scan_code,
            is_released: Some(released),
            is_extended: self.is_extended,
        }
    }

    pub const fn set_extended(self, extended: bool) -> Self {
        Self {
            scan_code: self.scan_code,
            is_released: self.is_released,
            is_extended: Some(extended),
        }
    }

    pub fn build(self) -> ScanCode {
        ScanCode {
            scan_code: self.scan_code.unwrap(),
            is_released: self.is_released.unwrap(),
            is_extended: self.is_extended.unwrap(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ScanCode {
    pub scan_code: u8,
    is_released: bool,
    is_extended: bool,
}

pub struct Keyboard {
    commands: LinkedQueue<basic::Commands>,
    incoming_bytes: LinkedQueue<u8>,
    state: State,
}

impl Keyboard {
    pub const fn new() -> Self {
        Self {
            commands: LinkedQueue::new(),
            state: State::Idle,
            incoming_bytes: LinkedQueue::new(),
        }
    }

    fn process_byte(&mut self, byte: u8) {
        self.incoming_bytes.push(byte);
    }

    fn send_command(command: u8) {
        CONTROLLER.with_move(|controller| {
            let controller = controller.map(|controller| {
                WaitingToWriteTrait::<u8>::block_until_ready(controller.as_writer())
                    .write(Into::<u8>::into(command))
            });
            (controller, ())
        });
    }
}

impl PS2Device for Keyboard {
    fn received_byte(&mut self, byte: u8) {
        self.process_byte(byte);
    }

    fn periodic(&mut self) {
        match self.state {
            State::Idle => {
                if self.commands.get_head().is_some() {
                    self.state = State::CommandReady;
                } else if !self.incoming_bytes.is_empty() {
                    match self.incoming_bytes.remove_head() {
                        0xE0 => {
                            self.state = State::ReceivedExtenededCode(
                                ScanCodeBuilder::new().set_extended(true),
                            );
                        }
                        0xF0 => {
                            self.state = State::ReceivedReleasedCode(
                                ScanCodeBuilder::new()
                                    .set_released(true)
                                    .set_extended(false),
                            );
                        }
                        byte => {
                            self.state = State::ReceivedScanCode(
                                ScanCodeBuilder::new()
                                    .set_code(byte)
                                    .set_extended(false)
                                    .set_released(false)
                                    .build(),
                            );
                        }
                    }
                }
            }
            State::ReceivedScanCode(scan_code) => {
                if !scan_code.is_released {
                    SCANCODE_BUFFER.with_mut_ref(|buffer| buffer.push(scan_code));
                }

                self.state = State::Idle;
            }
            State::CommandReady => {
                Self::send_command(u8::from(*self.commands.get_head().unwrap()));
                self.state = State::WaitingForResponse;
            }
            State::WaitingForResponse => todo!(),
            State::GotResponse(response) => match response {
                0xEE => {
                    println!("echo");
                    self.state = State::Idle;
                }
                _ => println!("invalid response: {response}"),
            },
            State::ReceivedReleasedCode(scan_code_builder) => {
                self.state = State::ReceivedScanCode(
                    scan_code_builder
                        .set_code(self.incoming_bytes.remove_head())
                        .build(),
                );
            }
            State::ReceivedExtenededCode(scan_code_builder) => {
                let new_byte = self.incoming_bytes.remove_head();

                if new_byte == 0xF0 {
                    self.state = State::ReceivedReleasedCode(scan_code_builder.set_released(true));
                } else {
                    self.state = State::ReceivedScanCode(
                        scan_code_builder
                            .set_released(false)
                            .set_code(new_byte)
                            .build(),
                    );
                }
            }
        }
    }
}

//
// mod commands {
//     use super::TypeList2;
//
//     pub(super) trait Response {}
//     pub(super) trait Command {
//         fn command_byte(&self) -> u8;
//         fn process_byte(&self, value: u8) -> u8;
//     }
//
//     /// Command received successfully
//     pub struct Ok;
//
//     /// Some issue with the last command sent
//     pub struct Resend;
//
//     pub type OkOrResend = TypeList2<Ok, Resend>;
//
//     // impl Response for OkOrResend {}
//
//     pub struct SetLed {
//         led_state: u8,
//     }
//
//     pub struct Command2<T: Command> {
//         command: T,
//         data: u8,
//         output: u8,
//     }
//
//     impl Command for SetLed {
//         // type Response = OkOrResend;
//
//         fn command_byte(&self) -> u8 {
//             0xED
//         }
//     }
//
//     pub struct Echo;
//
//     pub struct EchoResponse;
//
//     impl Command for Echo {
//
//         fn command_byte(&self) -> u8 {
//             0xEE
//         }
//     }
// }

pub enum TypeList2<A, B> {
    A(A),
    B(B),
}

mod basic {
    use super::TypeList2;

    #[derive(Copy, Clone, Debug)]
    pub enum Commands {
        SetLed(u8),
        Echo,
        GetOrSetScanCode(u8),
    }

    impl From<Commands> for u8 {
        fn from(value: Commands) -> Self {
            match value {
                Commands::SetLed(_) => 0xED,
                Commands::Echo => 0xEE,
                Commands::GetOrSetScanCode(_) => 0xF0,
            }
        }
    }

    impl Commands {
        pub fn respone(&self) -> TypeList2<TypeList2<Ack, Resend>, TypeList2<Echo, Resend>> {
            match self {
                Self::SetLed(_) => TypeList2::A(TypeList2::A(Ack)),
                Self::Echo => todo!(),
                Self::GetOrSetScanCode(_) => todo!(),
            }
        }
    }

    pub struct Ack;
    pub struct Resend;
    pub struct Echo;

    // pub enum Responses {
    //     ACK = 0xFA,
    //     Resend = 0xFE,
    //     Echo = 0xEE,
    // }
}
