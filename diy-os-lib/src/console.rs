use core::fmt::{self, Write};
use core::ops::DerefMut;

use crate::framebuffer::FRAME_BUFER;
use crate::serial::print as print_serial;

pub mod font;
pub mod graphics;

#[doc(hidden)]
pub fn print(args: fmt::Arguments) {
    if let Some(frame_buffer) = FRAME_BUFER.acquire().deref_mut() {
        let _ = frame_buffer.write_fmt(args);
    }
    print_serial(args);
}

#[doc(hidden)]
#[allow(clippy::missing_const_for_fn)]
#[allow(unused_variables)]
pub fn debug_print(args: fmt::Arguments) {
    #[cfg(feature = "debug")]
    print(args);
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::console::print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crete:: $crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[macro_export]
macro_rules! debug_print {
    ($($arg:tt)*) => ($crate::console::debug_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! debug_println {
    () => ($crete:: $crate::debug_print!("\n"));
    ($($arg:tt)*) => ($crate::debug_print!("{}\n", format_args!($($arg)*)));
}
