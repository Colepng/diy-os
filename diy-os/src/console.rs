use core::fmt::{self, Write};
use core::ops::DerefMut;

use crate::framebuffer::FRAME_BUFER;
use crate::serial::_print as _print_serial;

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    if let Some(frame_buffer) = FRAME_BUFER.acquire().deref_mut() {
        let _ = frame_buffer.write_fmt(args);
    }
    _print_serial(args);
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::console::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crete:: $crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}
