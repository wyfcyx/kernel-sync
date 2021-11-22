extern crate alloc;
use alloc::collections::linked_list::LinkedList;
use core::cell::{RefCell, UnsafeCell};
use core::future::Future;
use core::ops::{Deref, DerefMut};
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, Ordering};
use core::task::{Context, Poll, Waker};

use crate::spinlock::SpinLock;

/// A mutual exclusion and asynchronous primitive which could work
/// in bare metal environments.
///
/// This mutex block coroutine waiting for the lock to become available.
/// The mutex can also be statically initialized or created via a new
/// constructor.  Each mutex has a type parameter which represents the
/// data that it is protecting. The data can only be accessed through
/// the RAII guards returned from lock and try_lock, which guarantees
/// that the data is only ever accessed when the mutex is locked.
pub struct Mutex<T: ?Sized> {
    state: AtomicBool,
    wakers: SpinLock<LinkedList<Waker>>,
    data: UnsafeCell<T>,
}

/// An RAII implementation of a "scoped lock" of a mutex. When this structure is
/// dropped (falls out of scope), the lock will be unlocked.
///
/// The data protected by the mutex can be accessed through this guard via its
/// [`Deref`] and [`DerefMut`] implementations.
///
/// This structure is created by the [`lock`] and [`try_lock`] methods on
/// [`Mutex`].
///
/// [`lock`]: Mutex::lock
/// [`try_lock`]: Mutex::try_lock
#[must_use = "if unused the Mutex will immediately unlock"]
pub struct MutexGuard<'a, T: ?Sized> {
    mutex: &'a Mutex<T>,
}

/// A future which resolves when the target mutex has been successfully
/// acquired.
pub struct MutexLockFuture<'a, T: ?Sized> {
    mutex: &'a Mutex<T>,
}

unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}
unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}
unsafe impl<T: ?Sized + Send> Send for MutexGuard<'_, T> {}
unsafe impl<T: ?Sized + Sync> Sync for MutexGuard<'_, T> {}
unsafe impl<T: ?Sized + Send> Send for MutexLockFuture<'_, T> {}

impl<T> Mutex<T> {
    /// Creates a new mutex in an unlocked state ready for use.
    ///
    pub fn new(t: T) -> Self {
        Mutex {
            state: AtomicBool::new(false),
            wakers: SpinLock::new(LinkedList::new()),
            data: UnsafeCell::new(t),
        }
    }
}

impl<T: ?Sized> Mutex<T> {
    pub fn lock(&self) -> MutexLockFuture<'_, T> {
        return MutexLockFuture { mutex: self };
    }

    /// Attempts to acquire this lock immedidately.
    pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
        if !self.state.fetch_or(true, Ordering::Acquire) {
            Some(MutexGuard { mutex: self })
        } else {
            None
        }
    }

    pub fn unlock(&self) {
        self.state.store(false, Ordering::Release);
        let waker = self.wakers.lock().pop_front();
        if waker.is_some() {
            waker.unwrap().wake();
        }
    }

    pub fn register(&self, waker: Waker) {
        self.wakers.lock().push_back(waker);
    }
}

impl<'a, T: ?Sized> Future for MutexLockFuture<'a, T> {
    type Output = MutexGuard<'a, T>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(lock) = self.mutex.try_lock() {
            return Poll::Ready(lock);
        }
        let waker = cx.waker().clone();
        self.mutex.register(waker);
        if let Some(lock) = self.mutex.try_lock() {
            return Poll::Ready(lock);
        }
        Poll::Pending
    }
}

impl<T: ?Sized> Deref for MutexGuard<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        unsafe { &*self.mutex.data.get() }
    }
}

impl<T: ?Sized> DerefMut for MutexGuard<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.mutex.data.get() }
    }
}

impl<T: ?Sized> Drop for MutexGuard<'_, T> {
    #[inline]
    fn drop(&mut self) {
        self.mutex.unlock();
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use alloc::sync::Arc;
    use tokio;

    async fn handle(x: Arc<super::Mutex<i32>>, loop_cnt: i32) {
        for _ in 0..loop_cnt {
            let mut guard = x.lock().await;
            *guard += 1;
        }
    }

    #[tokio::test]
    async fn mutex_test() {
        let x = Arc::new(super::Mutex::new(0));
        let coroutine_cnt = 10;
        let loop_cnt = 500;
        let mut coroutines = vec![];
        for _ in 0..coroutine_cnt {
            let x_cloned = x.clone();
            coroutines.push(tokio::spawn(handle(x_cloned, loop_cnt)));
        }
        for coroutine in coroutines {
            tokio::join!(coroutine).0.unwrap();
        }
        assert_eq!(*x.lock().await, coroutine_cnt * loop_cnt);
    }
}
