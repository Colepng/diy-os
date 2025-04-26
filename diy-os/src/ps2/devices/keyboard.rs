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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    pub const fn build(self) -> ScanCode {
        ScanCode {
            scan_code: self.scan_code.unwrap(),
            is_released: self.is_released.unwrap(),
            is_extended: self.is_extended.unwrap(),
            set: self.scan_code_set.unwrap(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ScanCode {
    pub scan_code: u8,
    is_released: bool,
    #[allow(dead_code)]
    is_extended: bool,
    set: ScanCodeSet,
}

impl From<ScanCode> for Keycode {
    fn from(value: ScanCode) -> Self {
        assert!(value.set == ScanCodeSet::Set2, "only scan set 2 is Implemented");

        match value.scan_code {
            0x01 => Self::F9,
            0x03 => Self::F5,
            0x04 => Self::F3,
            0x05 => Self::F1,
            0x06 => Self::F2,
            0x07 => Self::F12,
            0x09 => Self::F10,
            0x0A => Self::F8,
            0x0B => Self::F6,
            0x0C => Self::F4,
            0x0D => Self::Tab,
            0x0E => Self::Grave,
            0x11 => todo!(), // left alt
            0x12 => Self::Shift,
            0x14 => todo!(), // left ctrl
            0x15 => Self::Q,
            0x16 => Self::One,
            0x1A => Self::Z,
            0x1B => Self::S,
            0x1C => Self::A,
            0x1D => Self::W,
            0x1E => Self::Two,
            0x21 => Self::C,
            0x22 => Self::X,
            0x23 => Self::D,
            0x24 => Self::E,
            0x25 => Self::Four,
            0x26 => Self::Three,
            0x29 => Self::Space,
            0x2A => Self::V,
            0x2B => Self::F,
            0x2C => Self::T,
            0x2D => Self::R,
            0x2E => Self::Five,
            0x31 => Self::N,
            0x32 => Self::B,
            0x33 => Self::H,
            0x34 => Self::G,
            0x35 => Self::Y,
            0x36 => Self::Six,
            0x3A => Self::M,
            0x3B => Self::J,
            0x3C => Self::U,
            0x3D => Self::Seven,
            0x3E => Self::Eight,
            0x41 => Self::Comma,
            0x42 => Self::K,
            0x43 => Self::I,
            0x44 => Self::O,
            0x45 => Self::Zero,
            0x46 => Self::Nine,
            0x49 => Self::Period,
            0x4A => Self::Backslash,
            0x4B => Self::L,
            0x4C => Self::Semicolon,
            0x4D => Self::P,
            0x4E => Self::Hyphen,
            0x5A => Self::Enter,
            _ => todo!(),
        }
    }
}

pub struct Keyboard {
    commands: LinkedQueue<Commands>,
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
