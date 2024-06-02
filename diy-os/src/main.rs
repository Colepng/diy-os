#![no_std]
#![no_main]
#![feature(optimize_attribute)]
#![feature(custom_test_frameworks)]
#![test_runner(diy_os::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![warn(clippy::pedantic, clippy::nursery, clippy::perf, clippy::style)]
#![deny(
    clippy::suspicious,
    clippy::correctness,
    clippy::complexity,
    clippy::missing_const_for_fn,
    unsafe_op_in_unsafe_fn
)]

extern crate alloc;

use bootloader_api::{
    config::{Mapping, Mappings},
    entry_point, BootInfo, BootloaderConfig,
};
use core::{borrow::BorrowMut, ops::DerefMut, panic::PanicInfo, usize};
use diy_os::{
    allocator, console::graphics::{Color, GraphicBackend}, framebuffer, hlt_loop, init, memory::{self, BootInfoFrameAllocator}, println, serial_println, timer::{self, sleep}
};

static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    let mut mappings = Mappings::new_default();
    mappings.physical_memory = Some(Mapping::Dynamic);
    config.mappings = mappings;

    config
};

entry_point!(main, config = &BOOTLOADER_CONFIG);

#[no_mangle]
extern "Rust" fn main(mut boot_info: &'static mut BootInfo) -> ! {
    boot_info = init(boot_info);

    let offset_addr =
        x86_64::VirtAddr::new(boot_info.physical_memory_offset.into_option().unwrap());

    // setup the heap
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_regions) };
    let mut mapper = unsafe { memory::init(offset_addr) };
    allocator::setup_heap(&mut mapper, &mut frame_allocator).expect("Failed to setup heap fuck u");

    println!("Hello, world!");

    let mut width = 0;
    let mut height = 0;
    if let Some(buffer) = framebuffer::FRAME_BUFER.acquire().deref_mut() {
        buffer.clear();
        width = buffer.get_width();
        height = buffer.get_hight();
        framebuffer::FRAME_BUFER.release();
    }


    let size = 500;
    let top_x = (width - height)/2;
    // serial_println!("height: {}", height);
    // serial_println!("{}", top_x);
    let mut squere = Rectangle::new(Point::new(top_x, 0), Point::new(height + top_x - 1,  height - 1));


    let mut thing = false;

    loop {
        if thing {
            draw_squre(&squere, Color::BLACK);
            squere.grow(10);
        } else {
            draw_squre(&squere, Color::WHITE);
            let res = squere.shrink(10);
        }
        
        if squere.top_left.x >= squere.bottom_right.x {
            thing = true;
        } else if squere.top_left.x == top_x {
            thing = false;
        }
    
        sleep(75);
    }
    
    // println!("going to sleep");
    //
    // timer::sleep(1000);
    //
    // println!("wakign up");

    hlt_loop();
}

struct Rectangle {
    top_left: Point,
    bottom_right: Point,
} 

impl Rectangle {
    fn new(top_left: Point, bottom_right: Point) -> Self {
        Self {top_left, bottom_right}
    }

    fn grow(&mut self, amount: usize) {
        self.top_left.x = self.top_left.x.saturating_sub(amount);
        self.top_left.y = self.top_left.y.saturating_sub(amount);

        self.bottom_right.x = self.bottom_right.x.saturating_add(amount);
        self.bottom_right.y = self.bottom_right.y.saturating_add(amount);
    }

    fn shrink(&mut self, amount: usize) -> bool {
        let mut overflow;
        (self.top_left.x, overflow) = self.top_left.x.overflowing_add(amount);

        if overflow {
            return overflow;
        }

        (self.top_left.y, overflow) = self.top_left.y.overflowing_add(amount);

        if overflow {
            return overflow;
        }

        (self.bottom_right.x, overflow) = self.bottom_right.x.overflowing_sub(amount);

        if overflow {
            return overflow;
        }

        (self.bottom_right.y, overflow) = self.bottom_right.y.overflowing_sub(amount);

        return overflow;
    }
}


#[derive(Clone, Copy, PartialEq, Eq)]
struct Point {
    x: usize,
    y: usize
}

impl Point {
    fn new(x: usize, y: usize) -> Self {
        Self { x, y, } 
    }
}

fn draw_squre(rectangle: &Rectangle, color: Color) {
    if let Some(framebuffer) = framebuffer::FRAME_BUFER.acquire().deref_mut() {
        for i in rectangle.top_left.x..=rectangle.bottom_right.x {
            framebuffer.plot_pixel(i, rectangle.top_left.y, color);
            framebuffer.plot_pixel(i, rectangle.bottom_right.y, color);
        }

        for i in rectangle.top_left.y..=rectangle.bottom_right.y {
            framebuffer.plot_pixel(rectangle.top_left.x, i, color);
            framebuffer.plot_pixel(rectangle.bottom_right.x, i, color);
        }
    };
}

/// This function is called on panic.
#[cfg(not(test))] // new attribute
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    diy_os::test_panic_handler(info)
}

// test to make sure tests won't panic
#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}
