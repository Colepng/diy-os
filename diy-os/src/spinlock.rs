use core::{
    cell::UnsafeCell,
    fmt,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

pub struct Spinlock<T: ?Sized> {
    locked: AtomicBool,
    interrupts_enabled: UnsafeCell<Option<bool>>,
    data: UnsafeCell<T>,
}

impl<T> Spinlock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            interrupts_enabled: UnsafeCell::new(None),
            data: UnsafeCell::new(data),
        }
    }
}

impl<T: ?Sized> Spinlock<T> {
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

    fn enable_interrupts(&self) {
        unsafe {
            if (*self.interrupts_enabled.get()) == Some(true) {
                x86_64::instructions::interrupts::enable();
            }
        }
    }

    fn disable_interrupts(&self) {
        unsafe {
            *self.interrupts_enabled.get() = Some(x86_64::instructions::interrupts::are_enabled());
        }

        x86_64::instructions::interrupts::disable();
    }

    /// Release the lock and enable interrupts.
    pub fn release(&self) {
        self.locked.store(false, Ordering::Release);
        self.enable_interrupts();
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
