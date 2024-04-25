use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::ptr;
use std::ptr::NonNull;
use std::sync::atomic::Ordering::{AcqRel, Acquire, Release};
use std::sync::atomic::{AtomicBool, AtomicPtr};
use std::sync::{Arc, LockResult};

pub struct MSCLock<T> {
    inner: MSCLockInner,
    data: UnsafeCell<T>,
}

unsafe impl<T> Send for MSCLock<T> {}

unsafe impl<T> Sync for MSCLock<T> {}

impl<T> MSCLock<T> {
    pub fn new(value: T) -> Self {
        MSCLock {
            inner: MSCLockInner::new(),
            data: UnsafeCell::new(value),
        }
    }

    pub fn lock(&self) -> LockResult<MSCLockGuard<'_, T>> {
        let record = self.inner.lock();
        Ok(MSCLockGuard::new(self, record))
    }

    fn unlock(&self, record: Arc<Record>) {
        self.inner.unlock(record);
    }
}

struct MSCLockInner {
    tail: AtomicPtr<Record>,
}

impl MSCLockInner {
    fn new() -> MSCLockInner {
        MSCLockInner {
            tail: AtomicPtr::new(ptr::null_mut()),
        }
    }

    fn lock(&self) -> Arc<Record> {
        let record = Arc::new(Record::new());
        let record_ptr = record.deref() as *const Record as *mut Record;
        let old = self.tail.swap(record_ptr, AcqRel);
        if old.is_null() {
            return record;
        }
        record.locked.store(true, Release);
        unsafe {
            (*old).next.store(record_ptr, Release);
        }
        while record.locked.load(Acquire) {
            std::hint::spin_loop()
        }
        record
    }

    fn unlock(&self, record: Arc<Record>) {
        let record_ptr = record.as_ref() as *const Record as *mut Record;
        let mut next_ptr = record.next.load(Acquire);
        if next_ptr.is_null() {
            if self
                .tail
                .compare_exchange(record_ptr, ptr::null_mut(), AcqRel, Acquire)
                .is_ok()
            {
                return;
            }
            while next_ptr.is_null() {
                next_ptr = record.next.load(Acquire)
            }
        }
        let next_node = unsafe { NonNull::new(next_ptr).unwrap().as_ref() };
        next_node.locked.store(false, Release);
    }
}

pub(crate) struct Record {
    pub locked: AtomicBool,
    pub next: AtomicPtr<Record>,
}

impl Record {
    pub fn new() -> Record {
        Record {
            locked: AtomicBool::new(false),
            next: AtomicPtr::new(ptr::null_mut()),
        }
    }

    pub fn reset(&mut self) {
        self.locked.store(false, Release);
        self.next.store(ptr::null_mut(), Release);
    }

    pub fn lock(&self) {
        self.locked.store(true, Release);
    }
}

pub struct MSCLockGuard<'a, T> {
    lock: &'a MSCLock<T>,
    record: Arc<Record>,
}

impl<T> MSCLockGuard<'_, T> {
    fn new(lock: &MSCLock<T>, record: Arc<Record>) -> MSCLockGuard<'_, T> {
        MSCLockGuard { lock, record }
    }
}

impl<T> Deref for MSCLockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for MSCLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for MSCLockGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.unlock(self.record.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn happy_path() {
        let n = 1000000;
        let thread_n = 8;
        let lock = Arc::new(MSCLock::new(0));
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
