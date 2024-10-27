use crate::{
    collections::queues::LinkedQueue,
    human_input_devices::{KEYMAP, Keycode},
    println,
    ps2::{
        CONTROLLER,
        controller::{InitalTrait, ReadyToWriteTrait, WaitingToWriteTrait},
    },
};

use super::PS2Device;

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
pub enum ScanCodeSet {
    Set1,
    Set2,
    Set3,
}

#[derive(Debug, Clone, Copy)]
pub struct ScanCodeBuilder {
    scan_code: Option<u8>,
    is_released: Option<bool>,
    is_extended: Option<bool>,
    scan_code_set: Option<ScanCodeSet>,
}

impl ScanCodeBuilder {
    pub const fn new() -> Self {
        Self {
            scan_code: None,
            is_released: None,
            is_extended: None,
            scan_code_set: None,
        }
    }

    pub const fn set_code(self, code: u8) -> Self {
        Self {
            scan_code: Some(code),
            is_released: self.is_released,
            is_extended: self.is_extended,
            scan_code_set: self.scan_code_set,
        }
    }

    pub const fn set_released(self, released: bool) -> Self {
        Self {
            scan_code: self.scan_code,
            is_released: Some(released),
            is_extended: self.is_extended,
            scan_code_set: self.scan_code_set,
        }
    }

    pub const fn set_extended(self, extended: bool) -> Self {
        Self {
            scan_code: self.scan_code,
            is_released: self.is_released,
            is_extended: Some(extended),
            scan_code_set: self.scan_code_set,
        }
    }

    pub const fn set_set(self, set: ScanCodeSet) -> Self {
        Self {
            scan_code: self.scan_code,
            is_released: self.is_released,
            is_extended: self.is_extended,
            scan_code_set: Some(set),
        }
    }

    pub fn build(self) -> ScanCode {
        ScanCode {
            scan_code: self.scan_code.unwrap(),
            is_released: self.is_released.unwrap(),
            is_extended: self.is_extended.unwrap(),
            scan_code_set: self.scan_code_set.unwrap(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ScanCode {
    pub scan_code: u8,
    is_released: bool,
    is_extended: bool,
    scan_code_set: ScanCodeSet,
}

impl From<ScanCode> for Keycode {
    fn from(value: ScanCode) -> Self {
        if !matches!(value.scan_code_set, ScanCodeSet::Set2) {
            panic!("kys");
        }
        match value.scan_code {
            0x01 => Keycode::F9,
            0x03 => Keycode::F5,
            0x04 => Keycode::F3,
            0x05 => Keycode::F1,
            0x06 => Keycode::F2,
            0x07 => Keycode::F12,
            0x09 => Keycode::F10,
            0x0A => Keycode::F8,
            0x0B => Keycode::F6,
            0x0C => Keycode::F4,
            0x0D => Keycode::Tab,
            0x0E => Keycode::Grave,
            0x11 => todo!(), // left alt
            0x12 => Keycode::Shift,
            0x14 => todo!(), // left ctrl
            0x15 => Keycode::Q,
            0x16 => Keycode::One,
            0x1A => Keycode::Z,
            0x1B => Keycode::S,
            0x1C => Keycode::A,
            0x1D => Keycode::D,
            0x1E => Keycode::Two,
            0x21 => Keycode::C,
            0x22 => Keycode::X,
            0x23 => Keycode::D,
            0x24 => Keycode::E,
            0x25 => Keycode::Four,
            0x26 => Keycode::Three,
            0x29 => Keycode::Space,
            0x2A => Keycode::V,
            0x2B => Keycode::F,
            0x2C => Keycode::T,
            0x2D => Keycode::R,
            0x2E => Keycode::Five,
            0x31 => Keycode::N,
            0x32 => Keycode::B,
            0x33 => Keycode::H,
            0x34 => Keycode::G,
            0x35 => Keycode::Y,
            0x36 => Keycode::Six,
            0x3A => Keycode::M,
            0x3B => Keycode::J,
            0x3C => Keycode::U,
            0x3D => Keycode::Seven,
            0x3E => Keycode::Eight,
            0x41 => Keycode::Comma,
            0x42 => Keycode::K,
            0x43 => Keycode::I,
            0x44 => Keycode::O,
            0x45 => Keycode::Zero,
            0x46 => Keycode::Nine,
            0x49 => Keycode::Period,
            0x4A => Keycode::Backslash,
            0x4B => Keycode::L,
            0x4C => Keycode::Semicolon,
            0x4D => Keycode::P,
            0x4E => Keycode::Hyphen,
            0x5A => Keycode::Enter,
            _ => todo!(),
        }
    }
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
                WaitingToWriteTrait::<u8>::block_until_ready(controller.into_writer())
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
                                ScanCodeBuilder::new()
                                    .set_extended(true)
                                    .set_set(ScanCodeSet::Set2),
                            );
                        }
                        0xF0 => {
                            self.state = State::ReceivedReleasedCode(
                                ScanCodeBuilder::new()
                                    .set_released(true)
                                    .set_set(ScanCodeSet::Set2)
                                    .set_extended(false),
                            );
                        }
                        byte => {
                            self.state = State::ReceivedScanCode(
                                ScanCodeBuilder::new()
                                    .set_code(byte)
                                    .set_extended(false)
                                    .set_released(false)
                                    .set_set(ScanCodeSet::Set2)
                                    .build(),
                            );
                        }
                    }
                }
            }
            State::ReceivedScanCode(scan_code) => {
                KEYMAP.with_mut_ref(|keymap| {
                    if scan_code.is_released {
                        keymap.release_key(Keycode::from(scan_code));
                    } else {
                        keymap.press_key(Keycode::from(scan_code));
                    }
                });

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
                        .set_set(ScanCodeSet::Set2)
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
                            .set_set(ScanCodeSet::Set2)
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
