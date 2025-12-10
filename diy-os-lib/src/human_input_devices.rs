use alloc::vec::Vec;
use core::mem::{Assume, TransmuteFrom};

use spinlock::Spinlock as Mutex;

use crate::{
    multitasking::sleep,
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
    LeftShift,
    RightShift,
    Tab,
    Grave,
    Comma,
    Period,
    QuestionMark,
    Semicolon,
    Apostrophe,
    LeftCurlyBracket,
    RightCurlyBracket,
    LeftSquareBracket,
    RightSquareBracket,
    Backslash,
    Slash,
    Capslock,
    LeftCtrl,
    LeftAlt,
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
    ScrollLock,
    KeypadNumlock,
    KeypadSlash,
    KeypadAstrisk,
    KeypadMinus,
    KeypadAdditon,
    KeypadPeriod,
    KeypadEnter,
    Keypad0,
    Keypad1,
    Keypad2,
    Keypad3,
    Keypad4,
    Keypad5,
    Keypad6,
    Keypad7,
    Keypad8,
    Keypad9,
    MultiMediaWWWSearch,
    MultiMediaWWWFavourites,
    MultiMediaWWWForward,
    MultiMediaWWWBack,
    MultiMediaWWWHome,
    MultiMediaWWWStop,
    MultiMediaPreviousTrack,
    MultiMediaNextTrack,
    MultiMediaVolumeDown,
    MultiMediaVolumeUp,
    MultiMediaMute,
    MultiMediaStop,
    MultiMediaPlayPause,
    MultiMediaCalculator,
    MultiMediaMyComputer,
    MultiMediaEmail,
    AcpiPower,
    AcpiSleep,
    AcpiWake,
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

#[derive(thiserror::Error, Debug)]
#[error("Keycode does not have a charcter representation")]
pub struct KeycodeToChar;

impl TryFrom<Keycode> for char {
    type Error = KeycodeToChar;

    #[allow(clippy::too_many_lines)]
    fn try_from(value: Keycode) -> Result<Self, KeycodeToChar> {
        match value {
            Keycode::A => Ok('A'),
            Keycode::B => Ok('B'),
            Keycode::C => Ok('C'),
            Keycode::D => Ok('D'),
            Keycode::E => Ok('E'),
            Keycode::F => Ok('F'),
            Keycode::G => Ok('G'),
            Keycode::H => Ok('H'),
            Keycode::I => Ok('I'),
            Keycode::J => Ok('J'),
            Keycode::K => Ok('K'),
            Keycode::L => Ok('L'),
            Keycode::M => Ok('M'),
            Keycode::N => Ok('N'),
            Keycode::O => Ok('O'),
            Keycode::P => Ok('P'),
            Keycode::Q => Ok('Q'),
            Keycode::R => Ok('R'),
            Keycode::S => Ok('S'),
            Keycode::T => Ok('T'),
            Keycode::U => Ok('U'),
            Keycode::V => Ok('V'),
            Keycode::W => Ok('W'),
            Keycode::X => Ok('X'),
            Keycode::Y => Ok('Y'),
            Keycode::Z => Ok('Z'),
            Keycode::One | Keycode::Keypad1 => Ok('1'),
            Keycode::Two | Keycode::Keypad2 => Ok('2'),
            Keycode::Three | Keycode::Keypad3 => Ok('3'),
            Keycode::Four | Keycode::Keypad4 => Ok('4'),
            Keycode::Five | Keycode::Keypad5 => Ok('5'),
            Keycode::Six | Keycode::Keypad6 => Ok('6'),
            Keycode::Seven | Keycode::Keypad7 => Ok('7'),
            Keycode::Eight | Keycode::Keypad8 => Ok('8'),
            Keycode::Nine | Keycode::Keypad9 => Ok('9'),
            Keycode::Zero | Keycode::Keypad0 => Ok('0'),
            Keycode::Hyphen | Keycode::KeypadMinus => Ok('-'),
            Keycode::Equal => Ok('='),
            Keycode::Space => Ok(' '),
            Keycode::Enter => Ok('\n'),
            Keycode::Tab => Ok('\t'),
            Keycode::Grave => Ok('`'),
            Keycode::Comma => Ok(','),
            Keycode::Period => Ok('.'),
            Keycode::QuestionMark => Ok('?'),
            Keycode::Semicolon => Ok(';'),
            Keycode::Apostrophe => Ok('\''),
            Keycode::LeftCurlyBracket => Ok('{'),
            Keycode::RightCurlyBracket => Ok('}'),
            Keycode::Backslash => Ok('\\'),
            Keycode::KeypadSlash | Keycode::Slash => Ok('/'),
            Keycode::KeypadAstrisk => Ok('*'),
            Keycode::KeypadAdditon => Ok('+'),
            _ => Err(KeycodeToChar),
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
