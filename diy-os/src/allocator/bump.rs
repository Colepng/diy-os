use core::{
    alloc::{GlobalAlloc, Layout},
    ptr,
};

use super::{align_up, Locked};

pub struct BumpAllocator {
    heap_start: *mut u8,
    heap_end_addr: usize,
    next_allocation: *mut u8,
    allocations: usize,
}

impl BumpAllocator {
    /// Creates a new empty bump allocator.
    pub const fn new() -> Self {
        Self {
            heap_start: ptr::null_mut(),
            heap_end_addr: 0,
            next_allocation: ptr::null_mut(),
            allocations: 0,
        }
    }

    /// Initializes the bump allocator with the given heap bounds.
    ///
    /// # Safety
    /// This method is unsafe because the caller must ensure that the given
    /// memory range is unused. Also, this method must be called only once.
    pub unsafe fn init(&mut self, heap_start: *mut u8, heap_size: usize) {
        self.heap_start = heap_start;
        self.heap_end_addr = heap_start.addr() + heap_size;
        self.next_allocation = heap_start;
    }
}

unsafe impl GlobalAlloc for Locked<BumpAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut bump = self.lock(); // get a mutable reference

        let alloc_start_addr = align_up(bump.next_allocation.addr(), layout.align());
        let Some(alloc_end) = alloc_start_addr.checked_add(layout.size()) else {
            return ptr::null_mut();
        };

        if alloc_end > bump.heap_end_addr {
            ptr::null_mut() // out of memory
        } else {
            bump.next_allocation = bump.next_allocation.with_addr(alloc_end);
            bump.allocations += 1;
            bump.heap_start.with_addr(alloc_start_addr)
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        let mut bump = self.lock(); // get a mutable reference

        bump.allocations -= 1;
        if bump.allocations == 0 {
            bump.next_allocation = bump.heap_start;
        }
    }
}

impl Default for BumpAllocator {
    fn default() -> Self {
        Self::new()
    }
}
