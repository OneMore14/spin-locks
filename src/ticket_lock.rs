use crate::lock_guard::lock_guard;
use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::AtomicI32;
use std::sync::atomic::Ordering::{AcqRel, Acquire};
use std::sync::LockResult;

pub struct TicketLock<T> {
    inner: TicketLockInner,
    data: UnsafeCell<T>,
}

unsafe impl<T> Send for TicketLock<T> {}

unsafe impl<T> Sync for TicketLock<T> {}

impl<T> TicketLock<T> {
    pub fn new(value: T) -> Self {
        TicketLock {
            inner: TicketLockInner::new(),
            data: UnsafeCell::new(value),
        }
    }

    pub fn lock(&self) -> LockResult<TicketLockGuard<'_, T>> {
        self.inner.lock();
        Ok(TicketLockGuard::new(self))
    }

    fn unlock(&self) {
        self.inner.unlock();
    }
}

struct TicketLockInner {
    head: AtomicI32,
    tail: AtomicI32,
}

impl TicketLockInner {
    fn new() -> Self {
        TicketLockInner {
            head: AtomicI32::new(0),
            tail: AtomicI32::new(0),
        }
    }

    fn lock(&self) {
        let ticket = self.tail.fetch_add(1, AcqRel);
        while self.head.load(Acquire) != ticket {
            std::hint::spin_loop();
        }
    }

    fn unlock(&self) {
        self.head.fetch_add(1, AcqRel);
    }
}

lock_guard!(TicketLockGuard, TicketLock);

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn happy_path() {
        let n = 1000000;
        let thread_n = 8;
        let lock = Arc::new(TicketLock::new(0));
        let threads: Vec<_> = (0..thread_n)
            .map(|_| {
                let lock = lock.clone();
                thread::spawn(move || {
                    for _ in 0..n {
                        let mut value = lock.lock().unwrap();
                        *value -= 1;
                        *value += 1;
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
