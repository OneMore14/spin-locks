use std::cell::{RefCell, UnsafeCell};
use std::ops::{Deref, DerefMut};
use std::ptr;
use std::ptr::NonNull;
use std::sync::atomic::AtomicPtr;
use std::sync::atomic::Ordering::{AcqRel, Acquire, Release};
use std::sync::LockResult;

use crate::lock_guard::lock_guard;
use crate::mcs_lock::Record;

thread_local! {
    static RECORD: RefCell<Record> = RefCell::new(Record::new());
}

pub struct ThreadLocalMscLock<T> {
    inner: ThreadLocalMscLockInner,
    data: UnsafeCell<T>,
}

unsafe impl<T> Send for ThreadLocalMscLock<T> {}

unsafe impl<T> Sync for ThreadLocalMscLock<T> {}

impl<T> ThreadLocalMscLock<T> {
    pub fn new(value: T) -> Self {
        ThreadLocalMscLock {
            inner: ThreadLocalMscLockInner::new(),
            data: UnsafeCell::new(value),
        }
    }

    pub fn lock(&self) -> LockResult<ThreadLocalMscLockGuard<'_, T>> {
        self.inner.lock();
        Ok(ThreadLocalMscLockGuard::new(self))
    }

    fn unlock(&self) {
        self.inner.unlock();
    }
}
struct ThreadLocalMscLockInner {
    tail: AtomicPtr<Record>,
}

impl ThreadLocalMscLockInner {
    fn new() -> ThreadLocalMscLockInner {
        ThreadLocalMscLockInner {
            tail: AtomicPtr::new(ptr::null_mut()),
        }
    }

    fn lock(&self) {
        RECORD.with_borrow_mut(|r: &mut Record| r.reset());
        let record_ptr: *mut Record = RECORD.with_borrow_mut(|r: &mut Record| r as *mut Record);
        let old = self.tail.swap(record_ptr, AcqRel);
        if old.is_null() {
            return;
        }
        RECORD.with_borrow(|r: &Record| r.lock());
        unsafe {
            (*old).next.store(record_ptr, Release);
        }
        while RECORD.with_borrow(|r: &Record| r.locked.load(Acquire)) {
            std::hint::spin_loop()
        }
    }

    fn unlock(&self) {
        let record_ptr: *mut Record = RECORD.with_borrow_mut(|r: &mut Record| r as *mut Record);
        let mut next_ptr = RECORD.with_borrow(|r: &Record| r.next.load(Acquire));
        if next_ptr.is_null() {
            if self
                .tail
                .compare_exchange(record_ptr, ptr::null_mut(), AcqRel, Acquire)
                .is_ok()
            {
                return;
            }
            while next_ptr.is_null() {
                next_ptr = RECORD.with_borrow(|r: &Record| r.next.load(Acquire));
            }
        }
        let next_node = unsafe { NonNull::new(next_ptr).unwrap().as_ref() };
        next_node.locked.store(false, Release);
    }
}

lock_guard!(ThreadLocalMscLockGuard, ThreadLocalMscLock);

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::thread;

    use super::*;

    #[test]
    fn happy_path() {
        let n = 100000;
        let thread_n = 8;
        let lock = Arc::new(ThreadLocalMscLock::new(0));
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
