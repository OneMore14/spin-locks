use crate::lock_guard::lock_guard;
use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::{AcqRel, Acquire};
use std::sync::LockResult;

pub struct SpinLock<T> {
    inner: SpinLockInner,
    data: UnsafeCell<T>,
}

unsafe impl<T> Send for SpinLock<T> {}

unsafe impl<T> Sync for SpinLock<T> {}

impl<T> SpinLock<T> {
    pub fn new(data: T) -> Self {
        SpinLock {
            inner: SpinLockInner::new(),
            data: UnsafeCell::new(data),
        }
    }

    pub fn lock(&self) -> LockResult<SpinLockGuard<'_, T>> {
        self.inner.lock();
        Ok(SpinLockGuard::new(self))
    }

    fn unlock(&self) {
        self.inner.unlock();
    }
}

/// the classic spinlock
struct SpinLockInner {
    lock: AtomicBool,
}

impl SpinLockInner {
    fn new() -> Self {
        SpinLockInner {
            lock: AtomicBool::new(false),
        }
    }

    fn lock(&self) {
        loop {
            if self
                .lock
                .compare_exchange(false, true, AcqRel, Acquire)
                .is_err()
            {
                std::hint::spin_loop();
            } else {
                break;
            }
        }
    }

    fn unlock(&self) {
        self.lock
            .compare_exchange(true, false, AcqRel, Acquire)
            .unwrap();
    }
}

lock_guard!(SpinLockGuard, SpinLock);

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn happy_path() {
        let n = 1000000;
        let thread_n = 8;
        let lock = Arc::new(SpinLock::new(0));
        let threads: Vec<_> = (0..thread_n)
            .map(|_| {
                let lock = lock.clone();
                thread::spawn(move || {
                    for _ in 0..n {
                        let mut value = lock.lock().unwrap();
                        *value += 1;
                    }
                })
            })
            .collect();

        for t in threads {
            t.join().unwrap();
        }
        let value = lock.lock().unwrap();
        assert_eq!(*value, n * thread_n);
    }
}
