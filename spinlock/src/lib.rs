#![cfg_attr(not(test), no_std)]
#![feature(negative_impls)]
#![warn(
    clippy::pedantic,
    clippy::nursery,
    clippy::perf,
    clippy::style,
    clippy::todo
)]
#![deny(
    clippy::suspicious,
    clippy::correctness,
    clippy::complexity,
    clippy::missing_const_for_fn
)]

use core::{
    cell::UnsafeCell,
    fmt,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Debug)]
pub struct Spinlock<T: ?Sized> {
    locked: AtomicBool,
    #[cfg(not(test))]
    interrupts_enabled: UnsafeCell<Option<bool>>,
    data: UnsafeCell<T>,
}

impl<T> Spinlock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            #[cfg(not(test))]
            interrupts_enabled: UnsafeCell::new(None),
            data: UnsafeCell::new(data),
        }
    }
}

impl<T: ?Sized> Spinlock<T> {
    pub fn is_acquired(&self) -> bool {
        self.locked.load(Ordering::Acquire)
    }

    /// Acquires a lock and disables interrupts.
    pub fn acquire(&self) -> SpinlockGuard<'_, T> {
        self.disable_interrupts();
        // loops until not locked
        while self
            .locked
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::Acquire)
            .is_err()
        {}

        SpinlockGuard { spinlock: self }
    }

    pub fn try_acquire(&self) -> Option<SpinlockGuard<'_, T>> {
        self.disable_interrupts();

        if self
            .locked
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::Acquire)
            .is_ok()
        {
            Option::Some(SpinlockGuard { spinlock: self })
        } else {
            self.enable_interrupts();
            Option::None
        }
    }

    fn enable_interrupts(&self) {
        #[cfg(not(test))]
        unsafe {
            if (*self.interrupts_enabled.get()) == Some(true) {
                x86_64::instructions::interrupts::enable();
            }
        }
    }

    fn disable_interrupts(&self) {
        #[cfg(not(test))]
        unsafe {
            *self.interrupts_enabled.get() = Some(x86_64::instructions::interrupts::are_enabled());
        }

        #[cfg(not(test))]
        x86_64::instructions::interrupts::disable();
    }

    /// Release the lock and enable interrupts.
    pub fn release(&self) {
        self.locked.store(false, Ordering::Release);
        self.enable_interrupts();
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

impl<T: Sized> Spinlock<T> {
    pub fn with_move<F, R>(&self, f: F) -> R
    where
        F: FnOnce(T) -> (T, R),
    {
        let _guard = self.acquire();

        let locked_value = unsafe { self.data.get().read() };

        let (value, ret) = f(locked_value);

        unsafe {
            self.data.get().write(value);
        }

        ret
    }

    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }
}

// these are the only places where `T: Send` matters; all other
// functionality works fine on a single thread.
unsafe impl<T: ?Sized + Send> Send for Spinlock<T> {}
unsafe impl<T: ?Sized + Send> Sync for Spinlock<T> {}

pub struct SpinlockGuard<'a, T: ?Sized> {
    spinlock: &'a Spinlock<T>,
}

unsafe impl<T: ?Sized + Sync> Sync for SpinlockGuard<'_, T> {}
impl<T: ?Sized> !Send for SpinlockGuard<'_, T> {}

impl<T: ?Sized> Drop for SpinlockGuard<'_, T> {
    #[inline]
    fn drop(&mut self) {
        self.spinlock.release();
    }
}

impl<T: ?Sized> Deref for SpinlockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.spinlock.data.get() }
    }
}

impl<T: ?Sized> DerefMut for SpinlockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.spinlock.data.get() }
    }
}

impl<T: fmt::Debug + ?Sized> fmt::Debug for SpinlockGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T: fmt::Display + ?Sized> fmt::Display for SpinlockGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

#[derive(Debug)]
pub struct SpinlockWithCallback<T: ?Sized> {
    locked: AtomicBool,
    #[cfg(not(test))]
    interrupts_enabled: UnsafeCell<Option<bool>>,
    callback: fn(&'static str) -> (),
    data: UnsafeCell<T>,
}

impl<T> SpinlockWithCallback<T> {
    pub const fn new(data: T, callback: fn(&'static str) -> ()) -> Self {
        Self {
            locked: AtomicBool::new(false),
            #[cfg(not(test))]
            interrupts_enabled: UnsafeCell::new(None),
            callback,
            data: UnsafeCell::new(data),
        }
    }
}

impl<T: ?Sized> SpinlockWithCallback<T> {
    pub fn is_acquired(&self) -> bool {
        self.locked.load(Ordering::Acquire)
    }

    /// Acquires a lock and disables interrupts.
    pub fn acquire(&self) -> SpinlockGuardWithCallback<'_, T> {
        (self.callback)("a1");

        self.disable_interrupts();
        // loops until not locked
        while self
            .locked
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::Acquire)
            .is_err()
        {}

        (self.callback)("a2");
        SpinlockGuardWithCallback { spinlock: self }
    }

    pub fn try_acquire(&self) -> Option<SpinlockGuardWithCallback<'_, T>> {
        (self.callback)("t1");

        self.disable_interrupts();

        if self
            .locked
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::Acquire)
            .is_ok()
        {
            (self.callback)("t2");
            Option::Some(SpinlockGuardWithCallback { spinlock: self })
        } else {
            self.enable_interrupts();
            (self.callback)("t2");
            Option::None
        }
    }

    fn enable_interrupts(&self) {
        #[cfg(not(test))]
        unsafe {
            if (*self.interrupts_enabled.get()) == Some(true) {
                x86_64::instructions::interrupts::enable();
            }
        }
    }

    fn disable_interrupts(&self) {
        #[cfg(not(test))]
        unsafe {
            *self.interrupts_enabled.get() = Some(x86_64::instructions::interrupts::are_enabled());
        }

        #[cfg(not(test))]
        x86_64::instructions::interrupts::disable();
    }

    /// Release the lock and enable interrupts.
    pub fn release(&self) {
        (self.callback)("r1");
        self.locked.store(false, Ordering::Release);
        (self.callback)("r2");
        self.enable_interrupts();
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

impl<T: Sized> SpinlockWithCallback<T> {
    pub fn with_move<F, R>(&self, f: F) -> R
    where
        F: FnOnce(T) -> (T, R),
    {
        let _guard = self.acquire();

        let locked_value = unsafe { self.data.get().read() };

        let (value, ret) = f(locked_value);

        unsafe {
            self.data.get().write(value);
        }

        ret
    }

    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }
}

// these are the only places where `T: Send` matters; all other
// functionality works fine on a single thread.
unsafe impl<T: ?Sized + Send> Send for SpinlockWithCallback<T> {}
unsafe impl<T: ?Sized + Send> Sync for SpinlockWithCallback<T> {}

pub struct SpinlockGuardWithCallback<'a, T: ?Sized> {
    spinlock: &'a SpinlockWithCallback<T>,
}

unsafe impl<T: ?Sized + Sync> Sync for SpinlockGuardWithCallback<'_, T> {}
impl<T: ?Sized> !Send for SpinlockGuardWithCallback<'_, T> {}

impl<T: ?Sized> Drop for SpinlockGuardWithCallback<'_, T> {
    #[inline]
    fn drop(&mut self) {
        self.spinlock.release();
    }
}

impl<T: ?Sized> Deref for SpinlockGuardWithCallback<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.spinlock.data.get() }
    }
}

impl<T: ?Sized> DerefMut for SpinlockGuardWithCallback<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.spinlock.data.get() }
    }
}

impl<T: fmt::Debug + ?Sized> fmt::Debug for SpinlockGuardWithCallback<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T: fmt::Display + ?Sized> fmt::Display for SpinlockGuardWithCallback<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use std::thread::spawn;

    use super::Spinlock;

    #[test]
    pub fn threaded_test() {
        static SPINLOCK: Spinlock<u8> = Spinlock::new(1);

        for _ in 0..100 {
            let _ = spawn(|| {
                let mut guard = SPINLOCK.acquire();

                *guard += 1;
            })
            .join();
        }

        assert_eq!(*SPINLOCK.acquire(), 101)
    }
}
