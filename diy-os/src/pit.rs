use x86_64::instructions::port::{Port, PortWriteOnly};

use crate::spinlock::Spinlock;

/// Lets all agree to never touch this in anything but the main cpu thread
static mut PIT_TAKEN: bool = false;

/// Sleep counter
pub static SLEEP_COUNTER: Spinlock<u64> = Spinlock::new(0);

pub struct Pit {
    pub channel_0_port: ChannelPort,
    pub channel_1_port: ChannelPort,
    pub channel_2_port: ChannelPort,
    pub mode_port: CommandRegister,
}

impl Pit {
    const fn new() -> Self {
        Self {
            channel_0_port: ChannelPort(Port::new(0x40)),
            channel_1_port: ChannelPort(Port::new(0x41)),
            channel_2_port: ChannelPort(Port::new(0x42)),
            mode_port: CommandRegister::new(),
        }
    }

    pub const fn take() -> Option<Self> {
        if unsafe { !PIT_TAKEN } {
            Some(Self::new())
        } else {
            None
        }
    }

    pub fn give_back(_: Self) {
        unsafe { PIT_TAKEN = false }
    }
}

impl Default for Pit {
    fn default() -> Self {
        Self::new()
    }
}

/// A safe wrapper around the IO port
pub struct ChannelPort(Port<u8>);

impl ChannelPort {
    /// Reads a [`u8`] from the IO port.
    #[inline]
    pub fn read(&mut self) -> u8 {
        unsafe {
            self.0.read()
        }
    }

    /// Writes a [`u8`] to the IO port.
    #[inline]
    pub fn write(&mut self, value: u8) {
        unsafe {
            self.0.write(value);
        }
    }

    /// Reads a [`ReadBackStatusByte`] from the IO port.
    /// # Safety
    /// The caller must insure that the channel was sent a [`ReadBackCommand`] and has not been
    /// read yet.
    pub unsafe fn read_status_byte_unchecked(&mut self) -> ReadBackStatusByte {
        let value = self.read();

        ReadBackStatusByte(value)
    }
}

/// <https://wiki.osdev.org/Programmable_Interval_Timer#I.2FO_Ports>
pub struct CommandRegister(pub PortWriteOnly<u8>);

impl CommandRegister {
    const fn new() -> Self {
        Self(PortWriteOnly::new(0x43))
    }

    #[allow(private_bounds)]
    pub fn write<T: PitCommand>(&mut self, value: T) {
        let byte: u8 = value.into();
        unsafe { self.0.write(byte) };
    }
}

#[repr(transparent)]
pub struct ConfigureChannelCommand(u8);

impl ConfigureChannelCommand {
    pub const fn new(
        channel: Channel,
        access_mode: AccessMode,
        operating_mode: OperatingMode,
        bcd_binary_mode: BcdBinaryMode,
    ) -> Self {
        Self(channel as u8 | access_mode as u8 | operating_mode as u8 | bcd_binary_mode as u8)
    }
}

impl PitCommand for ConfigureChannelCommand {}

/// Only implements into to prevent accidental conversion from [`u8`] to [`CommandRegister`]
#[allow(clippy::from_over_into)]
impl Into<u8> for ConfigureChannelCommand {
    #[inline]
    fn into(self) -> u8 {
        self.0
    }
}

/// Marker trait to represent that this command that can be sent to the pit through [`CommandRegister`]
trait PitCommand: Into<u8> {}

// pub fn latch_count_value_command() {}
//
// pub fn read_back_command() { }

#[repr(transparent)]
///Bits         Usage
///7 and 6      Must be set for the read back command
///5            Latch count flag (0 = latch count, 1 = don't latch count)
///4            Latch status flag (0 = latch status, 1 = don't latch status)
///3            Read back timer channel 2 (1 = yes, 0 = no)
///2            Read back timer channel 1 (1 = yes, 0 = no)
///1            Read back timer channel 0 (1 = yes, 0 = no)
///0            Reserved (should be clear)
pub struct ReadBackCommand(u8);

#[derive(Default)]
pub struct ReadBackCommandBuilder(u8);

impl<'a> ReadBackCommandBuilder {
    pub const fn new() -> Self {
        Self(0b1100_0000)
    }

    /// If set will read from channel 0 and any other channels selected
    pub fn set_read_from_channel_0(&'a mut self, value: bool) -> &'a mut Self {
        self.0 &= u8::from(value) << 1;

        self
    }

    /// If set will read from channel 1 and any other channels selected
    pub fn set_read_from_channel_1(&'a mut self, value: bool) -> &'a mut Self {
        self.0 |= u8::from(value) << 2;

        self
    }

    /// If set will read from channel 2 and any other channels selected
    pub fn set_read_from_channel_2(&'a mut self, value: bool) -> &'a mut Self {
        self.0 |= u8::from(value) << 3;

        self
    }

    /// If set then the selected channels will their current count copied into their latch
    /// register. This is similar to [`LatchCountValueCommand`] except it works for multiple
    /// channels with one command.
    pub fn set_read_latch_status(&'a mut self, value: bool) -> &'a mut Self {
        self.0 |= u8::from(!value) << 4;

        self
    }

    /// If set then the next read on the selected channels will return a [`ReadBackStatusByte`]
    pub fn set_read_status_byte(&'a mut self, value: bool) -> &'a mut Self {
        self.0 |= u8::from(!value) << 5;

        self
    }

    /// Builds a [`ReadBackCommand`]
    pub const fn build(&self) -> ReadBackCommand {
        ReadBackCommand(self.0)
    }
}

impl PitCommand for ReadBackCommand {}

/// Only implements into to prevent accidental conversion from [`u8`] to [`ReadBackCommand`]
#[allow(clippy::from_over_into)]
impl Into<u8> for ReadBackCommand {
    #[inline]
    fn into(self) -> u8 {
        self.0
    }
}

/// Bit/s        Usage
/// 7            Output pin state
/// 6            Null count flags
/// 4 and 5      Access mode :
///                 0 0 = Latch count value command
///                 0 1 = Access mode: lobyte only
///                 1 0 = Access mode: hibyte only
///                 1 1 = Access mode: lobyte/hibyte
/// 1 to 3       Operating mode :
///                 0 0 0 = Mode 0 (interrupt on terminal count)
///                 0 0 1 = Mode 1 (hardware re-triggerable one-shot)
///                 0 1 0 = Mode 2 (rate generator)
///                 0 1 1 = Mode 3 (square wave generator)
///                 1 0 0 = Mode 4 (software triggered strobe)
///                 1 0 1 = Mode 5 (hardware triggered strobe)
///                 1 1 0 = Mode 2 (rate generator, same as 010b)
///                 1 1 1 = Mode 3 (square wave generator, same as 011b)
/// 0            BCD/Binary mode: 0 = 16-bit binary, 1 = four-digit BCD
#[repr(transparent)]
pub struct ReadBackStatusByte(u8);

impl ReadBackStatusByte {
    pub const fn get_bcd_binary_mode(&self) -> BcdBinaryMode {
        unsafe { BcdBinaryMode::from_u8_unchecked(self.0 & BcdBinaryMode::BITMASK) }
    }

    pub const fn get_operating_mode(&self) -> OperatingMode {
        unsafe { OperatingMode::from_u8_unchecked(self.0 & OperatingMode::BITMASK) }
    }

    pub const fn get_access_mode(&self) -> AccessMode {
        unsafe { AccessMode::from_u8_unchecked(self.0 & AccessMode::BITMASK) }
    }

    pub const fn get_reload_value_indicator(&self) -> ReloadValueIndicator {
        unsafe { ReloadValueIndicator::from_u8_unchecked(self.0 & ReloadValueIndicator::BITMASK) }
    }

    pub const fn get_output_pin_state(&self) -> OutputPinState {
        unsafe { OutputPinState::from_u8_unchecked(self.0 & OutputPinState::BITMASK) }
    }

    pub const fn get_raw_byte(&self) -> u8 {
        self.0
    }
}

#[repr(u8)]
pub enum OutputPinState {
    High = 0b1000_0000,
    Low = 0b0000_0000,
}

impl OutputPinState {
    const BITMASK: u8 = 0b1000_0000;

    /// # Safety
    /// Caller must make sure that value is a valid [`OutputPinState`] variant
    pub const unsafe fn from_u8_unchecked(value: u8) -> Self {
        unsafe { core::mem::transmute::<u8, Self>(value) }
    }
}

#[repr(u8)]
pub enum ReloadValueIndicator {
    ReloadValueWrittenOrModeCommandRegisterInitialized = 0b0100_0000,
    ReloadValueCopied = 0b0000_0000,
}

impl ReloadValueIndicator {
    const BITMASK: u8 = 0b0100_0000;

    /// # Safety
    /// Caller must make sure that value is a valid [`ReloadValueIndicator`] variant
    #[inline]
    pub const unsafe fn from_u8_unchecked(value: u8) -> Self {
        unsafe { core::mem::transmute::<u8, Self>(value) }
    }
}

#[repr(u8)]
pub enum Channel {
    Channel0 = 0b0000_0000,
    Channel1 = 0b0100_0000,
    Channel2 = 0b1000_0000,
}

impl Channel {
    const BITMASK: u8 = 0b1100_0000;

    /// # Safety
    /// Caller must make sure that value is a valid [`Channel`] variant
    #[inline]
    pub const unsafe fn from_u8_unchecked(value: u8) -> Self {
        unsafe { core::mem::transmute::<u8, Self>(value) }
    }
}

#[repr(u8)]
pub enum AccessMode {
    LowbyteOnly = 0b0001_0000,
    HighbyteOnly = 0b0010_0000,
    LowHighbyte = 0b0011_0000,
}

impl AccessMode {
    const BITMASK: u8 = 0b0011_0000;

    /// # Safety
    /// Caller must make sure that value is a valid [`AccessMode`] variant
    #[inline]
    pub const unsafe fn from_u8_unchecked(value: u8) -> Self {
        unsafe { core::mem::transmute::<u8, Self>(value) }
    }
}

#[repr(u8)]
pub enum OperatingMode {
    InterruptOnTerminalCount = 0b0000,
    HardwareRetriggerableOneShot = 0b0010,
    RateGenerator = 0b0100,
    SquareWaveGenerator = 0b0110,
    SoftwareTriggeredStrobe = 0b1000,
    HardwareTriggeredStrobe = 0b1010,
}

impl OperatingMode {
    const BITMASK: u8 = 0b1110;
    /// # Safety
    /// Caller must make sure that value is a valid [`OperatingMode`] variant
    #[inline]
    pub const unsafe fn from_u8_unchecked(value: u8) -> Self {
        unsafe { core::mem::transmute::<u8, Self>(value) }
    }
}

#[repr(u8)]
pub enum BcdBinaryMode {
    Binary16Bit = 0b0,
    FourDigitBcd = 0b1,
}

impl BcdBinaryMode {
    const BITMASK: u8 = 0b1;

    #[inline]
    pub const fn to_u8(self) -> u8 {
        self as u8
    }

    /// # Safety
    /// Caller must make sure that value is a valid [`BcdBinaryMode`] variant
    #[inline]
    pub const unsafe fn from_u8_unchecked(value: u8) -> Self {
        unsafe { core::mem::transmute::<u8, Self>(value) }
    }
}

impl TryFrom<u8> for BcdBinaryMode {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0b0 => Ok(Self::Binary16Bit),
            0b1 => Ok(Self::FourDigitBcd),
            _ => Err("u8 passed is a invalid BcdBinaryMode variant"),
        }
    }
}

impl From<BcdBinaryMode> for u8 {
    fn from(value: BcdBinaryMode) -> Self {
        value.to_u8()
    }
}

pub fn get_reload_value_from_frequency(frequency: u32) -> u16 {
    u16::try_from(1_192_182 / frequency).unwrap()
}

pub fn set_count(pit: &mut Pit, count: u16) -> &mut Pit {
    x86_64::instructions::interrupts::without_interrupts(|| {
        pit.channel_0_port.write((count & 0xFF).try_into().unwrap()); // low_byte
        pit.channel_0_port
            .write(((count & 0xFF00) >> 8).try_into().unwrap()); // high byte
    });

    pit
}
