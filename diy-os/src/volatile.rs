use core::marker::Send;
use core::marker::Copy;
use core::marker::Sized;
use core::assert;
use core::{marker::PhantomData, ptr::NonNull};

pub trait Access {}

pub struct WriteOnly;
pub struct ReadOnly;
pub struct ReadAndWrite;

impl Access for WriteOnly {}
impl Access for ReadOnly {}
impl Access for ReadAndWrite {}

// Smart pointer with the same semantics as a mutable ref but all access are volatile
pub struct VolatileMutRef<'a, T: ?Sized> {
    ptr: NonNull<T>,
    _lifetime: PhantomData<&'a mut T>,
}

impl<T: ?Sized> VolatileMutRef<'_, T> {
    /// ptr must be unique, non null, and initialized
    pub const fn new(ptr: NonNull<T>) -> Self {
        Self {
            ptr,
            _lifetime: PhantomData,
        }
    }

    pub const fn as_ptr(&mut self) -> NonNull<T> {
        self.ptr
    }
}

impl<T: Copy> VolatileMutRef<'static, T> {
    pub fn read(&mut self) -> T {
        assert!(self.ptr.is_aligned());

        // SAFETY:
        //      - T is required to impl copy
        //      - Ptr is aligned because of assert above
        //      - Ptr has to be initialized in the first place
        unsafe { self.ptr.read_volatile() }
    }

    pub fn write(&mut self, value: T) {
        // SAFETY:
        //      - T is required to impl copy
        //      - Ptr is aligned because of assert above
        unsafe {
            self.ptr.write_volatile(value);
        }
    }
}

impl<T: Copy> VolatileMutRef<'static, [T]> {
    /// Performs the indexing on container behind the pointer.
    /// # Panics
    ///
    /// May panic if the index is out of bounds.
    pub fn index(&mut self, index: usize) -> T {
        let element_x_ptr = unsafe { self.ptr.cast::<T>().add(index) };
        let len = self.len();
        assert!(index < len);
        assert!(element_x_ptr.is_aligned());

        // SAFETY:
        //      - T is required to impl copy
        //      - Ptr is aligned because of assert above
        //      - Ptr has to be initialized since it points into a slice
        unsafe { element_x_ptr.read_volatile() }
    }

    /// Performs the mutable indexing on the container behind the pointer.
    /// # Panics
    ///
    /// May panic if the index is out of bounds.
    pub fn index_mut(&mut self, index: usize, value: T) {
        let element_x_ptr = unsafe { self.ptr.cast::<T>().add(index) };
        let len = self.len();

        assert!(index < len);
        assert!(element_x_ptr.is_aligned());

        // SAFETY:
        //      - T is required to impl copy
        //      - Ptr is aligned because of assert above
        unsafe {
            element_x_ptr.write_volatile(value);
        }
    }

    pub const fn len(&mut self) -> usize {
        core::ptr::metadata(self.ptr.as_ptr())
    }

    pub const fn is_empty(&mut self) -> bool {
        self.len() == 0
    }
}

unsafe impl<T: ?Sized + Send> Send for VolatileMutRef<'_, T> {}

#[repr(transparent)]
pub struct VolatilePtr<'a, T: ?Sized, A: Access> {
    ptr: NonNull<T>,
    _lifetime: PhantomData<&'a mut T>,
    access: PhantomData<A>,
}

impl<T: ?Sized, A: Access> VolatilePtr<'_, T, A> {
    pub const fn new(reference: &mut T) -> Self {
        Self {
            ptr: unsafe { NonNull::new_unchecked(reference) },
            _lifetime: PhantomData,
            access: PhantomData,
        }
    }

    pub const fn as_ptr(&mut self) -> NonNull<T> {
        self.ptr
    }
}

impl<T> VolatilePtr<'_, T, ReadOnly> {
    pub fn read(&mut self) -> T {
        unsafe { self.ptr.read_volatile() }
    }
}

impl<T> VolatilePtr<'_, T, WriteOnly> {
    pub fn write(&mut self, value: T) {
        unsafe { self.ptr.write_volatile(value) };
    }
}

impl<T> VolatilePtr<'_, T, ReadAndWrite> {
    pub fn read(&mut self) -> T {
        unsafe { self.ptr.read_volatile() }
    }

    pub fn write(&mut self, value: T) {
        unsafe { self.ptr.write_volatile(value) };
    }
}
impl<T: Copy> VolatilePtr<'_, [T], ReadAndWrite> {
    /// Performs the indexing on container behind the pointer.
    /// # Panics
    ///
    /// May panic if the index is out of bounds.
    pub fn index(&mut self, index: usize) -> T {
        let element_x_ptr = unsafe { self.ptr.cast::<T>().add(index) };
        let len = self.len();
        assert!(index < len);
        assert!(element_x_ptr.is_aligned());

        // SAFETY:
        //      - T is required to impl copy
        //      - Ptr is aligned because of assert above
        //      - Ptr has to be initialized since it points into a slice
        unsafe { element_x_ptr.read_volatile() }
    }

    /// Performs the mutable indexing on the container behind the pointer.
    /// # Panics
    ///
    /// May panic if the index is out of bounds.
    pub fn index_mut(&mut self, index: usize, value: T) {
        let element_x_ptr = unsafe { self.ptr.cast::<T>().add(index) };
        let len = self.len();

        assert!(index < len);
        assert!(element_x_ptr.is_aligned());

        // SAFETY:
        //      - T is required to impl copy
        //      - Ptr is aligned because of assert above
        unsafe {
            element_x_ptr.write_volatile(value);
        }
    }

    pub const fn len(&mut self) -> usize {
        core::ptr::metadata(self.ptr.as_ptr())
    }

    pub const fn is_empty(&mut self) -> bool {
        self.len() == 0
    }
}

unsafe impl<T: ?Sized, A: Access> Send for VolatilePtr<'_, T, A> {}
