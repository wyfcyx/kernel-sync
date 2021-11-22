use core::{
    cell::UnsafeCell,
    fmt,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

pub struct SpinLock<T: ?Sized> {
    // phantom: PhantomData<R>,
    pub(crate) locked: AtomicBool,
    cpuid: u8,
    data: UnsafeCell<T>,
}

/// An RAII implementation of a “scoped lock” of a mutex.
/// When this structure is dropped (falls out of scope),
/// the lock will be unlocked.
///
pub struct SpinLockGuard<'a, T: ?Sized + 'a> {
    lock: &'a AtomicBool,
    data: &'a mut T,
}

unsafe impl<T: ?Sized + Send> Sync for SpinLock<T> {}
unsafe impl<T: ?Sized + Send> Send for SpinLock<T> {}

impl<T> SpinLock<T> {
    #[inline(always)]
    pub const fn new(data: T) -> Self {
        SpinLock {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
            cpuid: 0,
        }
    }

    #[inline(always)]
    pub fn into_inner(self) -> T {
        // We know statically that there are no outstanding references to
        // `self` so there's no need to lock.
        let SpinLock { data, .. } = self;
        data.into_inner()
    }

    #[inline(always)]
    pub fn as_mut_ptr(&self) -> *mut T {
        self.data.get()
    }
}

impl<T: ?Sized> SpinLock<T> {
    #[inline(always)]
    pub fn lock(&self) -> SpinLockGuard<T> {
        unsafe {
            crate::enable_intr();
        }
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            // Wait until the lock looks unlocked before retrying
            while self.is_locked() {
                // R::relax();
            }
        }

        SpinLockGuard {
            lock: &self.locked,
            data: unsafe { &mut *self.data.get() },
        }
    }

    #[inline(always)]
    pub fn is_locked(&self) -> bool {
        self.locked.load(Ordering::Relaxed)
    }

    #[inline(always)]
    pub fn try_lock(&self) -> Option<SpinLockGuard<T>> {
        if self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Some(SpinLockGuard {
                lock: &self.locked,
                data: unsafe { &mut *self.data.get() },
            })
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn get_mut(&mut self) -> &mut T {
        // We know statically that there are no other references to `self`, so
        // there's no need to lock the inner mutex.
        unsafe { &mut *self.data.get() }
    }
}

impl<'a, T: ?Sized + fmt::Display> fmt::Display for SpinLockGuard<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<'a, T: ?Sized> Deref for SpinLockGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.data
    }
}

impl<'a, T: ?Sized> DerefMut for SpinLockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.data
    }
}

impl<'a, T: ?Sized> Drop for SpinLockGuard<'a, T> {
    /// The dropping of the MutexGuard will release the lock it was created from.
    fn drop(&mut self) {
        self.lock.store(false, Ordering::Release);
        unsafe {
            crate::disable_intr();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    #[test]
    fn basic_test() {
        let x = Arc::new(super::SpinLock::new(0));
        let thread_cnt = 3;
        let loop_cnt = 1000000;
        let mut threads = vec![];
        for _ in 0..thread_cnt {
            let x_clone = x.clone();
            threads.push(std::thread::spawn(move || {
                for _ in 0..loop_cnt {
                    let mut guard = x_clone.lock();
                    *guard += 1;
                }
            }));
        }
        for thread in threads {
            thread.join().unwrap();
        }
        assert_eq!(*(x.lock()), thread_cnt * loop_cnt);
    }
    #[test]
    fn try_lock_test() {
        let x = Arc::new(super::SpinLock::new(0));
        let lock_result0 = x.try_lock();
        assert!(lock_result0.is_some());

        let lock_result1 = x.try_lock();
        assert!(lock_result1.is_none());

        drop(lock_result0);

        let lock_result2= x.try_lock();
        assert!(lock_result2.is_some());
    }
}
