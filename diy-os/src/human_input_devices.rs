use alloc::vec::Vec;
use core::mem::{Assume, TransmuteFrom};

use crate::{
    multitasking::{mutex::Mutex, sleep},
    timer::{Duration, Miliseconds, TIME_KEEPER},
};

pub(crate) static KEYMAP: Mutex<Keymap> = Mutex::new(Keymap::new());

pub static STDIN: Mutex<Vec<Keycode>> = Mutex::new(Vec::new());

#[repr(u8)]
#[derive(Debug)]
pub enum Keycode {
    A = 0x0,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Zero,
    Hyphen,
    Equal,
    Space,
    Enter,
    Backspace,
    Delete,
    Escape,
    Shift,
    Tab,
    Grave,
    Comma,
    Period,
    QuestionMark,
    Semicolon,
    Apostrophe,
    LeftBracket,
    RightBracket,
    Backslash,
    Capslock,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
    Insert,
    PrintScreen,
    Home,
    End,
    PageUp,
    PageDown,
    ArrowLeft,
    ArrowUp,
    ArrowRight,
    ArrowDown,
    KeypadNumlock,
    KeypadSlash,
    KeypadAstrisk,
    KeypadMinus,
    KeypadAdditon,
    KeypadEnter,
    Keypad1,
    Keypad2,
    Keypad3,
    Keypad4,
    Keypad5,
    Keypad6,
    Keypad7,
    Keypad8,
    Keypad9,
}

impl Keycode {
    /// Creates [`Keycode`] from a usize
    ///
    /// # Safety
    ///
    /// Caller must insure num is a valid variant of [`Keycode`]
    /// # Panics
    ///
    /// Panics if safety was broken.
    ///
    pub unsafe fn from_usize_unchecked(num: usize) -> Self {
        const ASSUMPTIONS: Assume = Assume {
            alignment: false,
            lifetimes: false,
            safety: true,
            validity: true,
        };

        assert!(num < core::mem::variant_count::<Self>());

        // SAFETY
        //      Validity - all numbers less then the variant count are valid; Checked by the assert
        //      Safety - No safety invariant
        unsafe { <Self as TransmuteFrom<usize, { ASSUMPTIONS }>>::transmute(num) }
    }
}

impl From<Keycode> for usize {
    fn from(value: Keycode) -> Self {
        value as Self
    }
}

impl From<Keycode> for char {
    #[allow(clippy::too_many_lines)]
    fn from(value: Keycode) -> Self {
        match value {
            Keycode::A => 'A',
            Keycode::B => 'B',
            Keycode::C => 'C',
            Keycode::D => 'D',
            Keycode::E => 'E',
            Keycode::F => 'F',
            Keycode::G => 'G',
            Keycode::H => 'H',
            Keycode::I => 'I',
            Keycode::J => 'J',
            Keycode::K => 'K',
            Keycode::L => 'L',
            Keycode::M => 'M',
            Keycode::N => 'N',
            Keycode::O => 'O',
            Keycode::P => 'P',
            Keycode::Q => 'Q',
            Keycode::R => 'R',
            Keycode::S => 'S',
            Keycode::T => 'T',
            Keycode::U => 'U',
            Keycode::V => 'V',
            Keycode::W => 'W',
            Keycode::X => 'X',
            Keycode::Y => 'Y',
            Keycode::Z => 'Z',
            Keycode::One => '1',
            Keycode::Two => '2',
            Keycode::Three => '3',
            Keycode::Four => '4',
            Keycode::Five => '5',
            Keycode::Six => '6',
            Keycode::Seven => '7',
            Keycode::Eight => '8',
            Keycode::Nine => '9',
            Keycode::Zero => '0',
            Keycode::Hyphen => '-',
            Keycode::Equal => '=',
            Keycode::Space => ' ',
            Keycode::Enter => '\n',
            Keycode::Backspace => todo!(),
            Keycode::Delete => todo!(),
            Keycode::Escape => todo!(),
            Keycode::Shift => todo!(),
            Keycode::Tab => todo!(),
            Keycode::Grave => todo!(),
            Keycode::Comma => todo!(),
            Keycode::Period => todo!(),
            Keycode::QuestionMark => todo!(),
            Keycode::Semicolon => todo!(),
            Keycode::Apostrophe => todo!(),
            Keycode::LeftBracket => todo!(),
            Keycode::RightBracket => todo!(),
            Keycode::Backslash => todo!(),
            Keycode::Capslock => todo!(),
            Keycode::F1 => todo!(),
            Keycode::F2 => todo!(),
            Keycode::F3 => todo!(),
            Keycode::F4 => todo!(),
            Keycode::F5 => todo!(),
            Keycode::F6 => todo!(),
            Keycode::F7 => todo!(),
            Keycode::F8 => todo!(),
            Keycode::F9 => todo!(),
            Keycode::F10 => todo!(),
            Keycode::F11 => todo!(),
            Keycode::F12 => todo!(),
            Keycode::F13 => todo!(),
            Keycode::F14 => todo!(),
            Keycode::F15 => todo!(),
            Keycode::F16 => todo!(),
            Keycode::F17 => todo!(),
            Keycode::F18 => todo!(),
            Keycode::F19 => todo!(),
            Keycode::F20 => todo!(),
            Keycode::F21 => todo!(),
            Keycode::F22 => todo!(),
            Keycode::F23 => todo!(),
            Keycode::F24 => todo!(),
            Keycode::Insert => todo!(),
            Keycode::PrintScreen => todo!(),
            Keycode::Home => todo!(),
            Keycode::End => todo!(),
            Keycode::PageUp => todo!(),
            Keycode::PageDown => todo!(),
            Keycode::ArrowLeft => todo!(),
            Keycode::ArrowUp => todo!(),
            Keycode::ArrowRight => todo!(),
            Keycode::ArrowDown => todo!(),
            Keycode::KeypadNumlock => todo!(),
            Keycode::KeypadSlash => todo!(),
            Keycode::KeypadAstrisk => todo!(),
            Keycode::KeypadMinus => todo!(),
            Keycode::KeypadAdditon => todo!(),
            Keycode::KeypadEnter => todo!(),
            Keycode::Keypad1 => todo!(),
            Keycode::Keypad2 => todo!(),
            Keycode::Keypad3 => todo!(),
            Keycode::Keypad4 => todo!(),
            Keycode::Keypad5 => todo!(),
            Keycode::Keypad6 => todo!(),
            Keycode::Keypad7 => todo!(),
            Keycode::Keypad8 => todo!(),
            Keycode::Keypad9 => todo!(),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum PressedState {
    Pressed,
    Released,
    Held,
}

#[derive(Copy, Clone)]
pub struct KeyState {
    pressed: PressedState,
    duration: Duration,
}

impl KeyState {
    pub const fn new() -> Self {
        Self {
            pressed: PressedState::Released,
            duration: Duration::new(),
        }
    }
}

pub struct Keymap {
    keys: [KeyState; core::mem::variant_count::<Keycode>()],
}

impl Keymap {
    pub const fn new() -> Self {
        Self {
            keys: [KeyState::new(); core::mem::variant_count::<Keycode>()],
        }
    }

    pub fn press_key(&mut self, code: Keycode) {
        let state = &mut self.keys[usize::from(code)];
        state.pressed = PressedState::Pressed;
    }

    pub fn release_key(&mut self, code: Keycode) {
        let state = &mut self.keys[usize::from(code)];
        state.pressed = PressedState::Released;
        state.duration.reset();
    }

    pub fn get_key_state(&self, code: Keycode) -> KeyState {
        self.keys[usize::from(code)]
    }

    pub fn get_pressed_keys(&self) -> impl Iterator<Item = Keycode> {
        self.keys.iter().enumerate().filter_map(|(index, state)| {
            if matches!(state.pressed, PressedState::Pressed | PressedState::Held) {
                // SAFETY:
                //      - Num has to be a valid variant since there is only variant count number of
                //      elements
                Some(unsafe { Keycode::from_usize_unchecked(index) })
            } else {
                None
            }
        })
    }

    pub fn get_pressed_keys_mut(&mut self) -> impl Iterator<Item = Keycode> {
        self.keys
            .iter_mut()
            .enumerate()
            .filter_map(|(index, state)| {
                if matches!(state.pressed, PressedState::Pressed | PressedState::Held) {
                    // SAFETY:
                    //      - Num has to be a valid variant since there is only variant count number of
                    //      elements
                    Some(unsafe { Keycode::from_usize_unchecked(index) })
                } else {
                    None
                }
            })
    }
}

pub fn process_keys() -> ! {
    loop {
        let duration = TIME_KEEPER.with_mut_ref(|keeper| {
            let dur = keeper.keyboard_counter.time;
            keeper.keyboard_counter.time.reset();
            dur
        });

        KEYMAP.with_mut_ref(|keymap| {
            let mut buffer = STDIN.acquire();

            keymap
                .keys
                .iter_mut()
                .enumerate()
                .filter(|(_, state)| {
                    matches!(state.pressed, PressedState::Pressed | PressedState::Held)
                })
                .for_each(|(index, state)| {
                    if state.duration == Duration::ZERO && state.pressed == PressedState::Pressed {
                        // println!("{}", state.duration);
                        // SAFETY:
                        //      - Num has to be a valid variant since there is only variant count number of
                        //      elements
                        buffer.push(unsafe { Keycode::from_usize_unchecked(index) });
                        state.duration += duration;
                        state.pressed = PressedState::Held;
                    } else if state.duration >= Duration::from(Miliseconds(600))
                        && state.pressed == PressedState::Held
                    {
                        // SAFETY:
                        //      - Num has to be a valid variant since there is only variant count number of
                        //      elements
                        buffer.push(unsafe { Keycode::from_usize_unchecked(index) });
                        state.duration = Duration::from(Miliseconds(50));
                    } else {
                        state.duration += duration;
                    }
                });
        });

        sleep(Miliseconds(10).into());
    }
}
