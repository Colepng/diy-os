use core::{
    cell::UnsafeCell,
    fmt,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

use crate::multitasking::schedule;

pub struct Mutex<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

impl<T> Mutex<T> {
    pub const fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    pub fn acquire(&self) -> MutexGuard<'_, T> {
        while self
            .locked
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::Acquire)
            .is_err()
        {
            unsafe { schedule(); };
        }

        MutexGuard { inner: self }
    }

    pub fn release(&self) {
        self.locked.store(false, Ordering::Release);
    }

    pub fn is_acquired(&self) -> bool {
        self.locked.load(Ordering::Acquire)
    }

    /// Runs a closure mutable referencing the locked value
    pub fn with_ref<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        let guard = self.acquire();

        f(&guard)
    }

    /// Runs a closure mutable referencing the locked value
    pub fn with_mut_ref<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self.acquire();

        f(&mut guard)
    }
}

unsafe impl<T: Send> Send for Mutex<T> { } 
unsafe impl<T: Send> Sync for Mutex<T> { }

pub struct MutexGuard<'a, T> {
    inner: &'a Mutex<T>,
}

impl<T> Drop for MutexGuard<'_, T> {
    #[inline]
    fn drop(&mut self) {
        self.inner.release();
    }
}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.inner.data.get().as_ref().unwrap() }
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.inner.data.get().as_mut().unwrap() }
    }
}

impl<T: fmt::Debug> fmt::Debug for MutexGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T: fmt::Display> fmt::Display for MutexGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}
