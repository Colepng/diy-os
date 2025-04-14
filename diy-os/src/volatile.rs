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
    ptr: *mut T,
    _lifetime: PhantomData<&'a mut T>,
}

impl<T: ?Sized> VolatileMutRef<'_, T> {
    /// ptr must be unique and non null and initialized
    pub const fn new(ptr: *mut T) -> Self {
        Self {
            ptr,
            _lifetime: PhantomData,
        }
    }

    pub const fn as_ptr(&mut self) -> *mut T {
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

    #[inline(always)]
    pub const fn len(&mut self) -> usize {
        core::ptr::metadata(self.ptr)
    }
}

unsafe impl<T: ?Sized + Send> Send for VolatileMutRef<'_, T> {}

#[repr(transparent)]
pub struct VolatilePtr<'a, T: ?Sized, A: Access> {
    ptr: NonNull<T>,
    _lifetime: PhantomData<&'a mut T>,
    access: PhantomData<A>,
}

impl<'a, T: ?Sized, A: Access> VolatilePtr<'a, T, A> {
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
    pub fn index(&mut self, index: usize) -> T {
        unsafe {
            self.ptr
                .byte_add(index * size_of::<T>())
                .cast::<T>()
                .read_volatile()
        }
    }

    pub fn index_mut(&mut self, index: usize, value: T) {
        unsafe {
            self.ptr
                .byte_add(index * size_of::<T>())
                .cast::<T>()
                .write_volatile(value);
        }
    }
}

unsafe impl<T: ?Sized, A: Access> Send for VolatilePtr<'_, T, A> {}
