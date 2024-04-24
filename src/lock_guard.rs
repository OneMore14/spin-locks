macro_rules! lock_guard {
    ($name: ident, $lock_name: ident) => {
        pub struct $name<'a, T> {
            lock: &'a $lock_name<T>,
        }

        impl<T> $name<'_, T> {
            fn new(lock: &$lock_name<T>) -> $name<'_, T> {
                $name { lock }
            }
        }

        impl<T> Deref for $name<'_, T> {
            type Target = T;

            fn deref(&self) -> &T {
                unsafe { &*self.lock.data.get() }
            }
        }

        impl<T> DerefMut for $name<'_, T> {
            fn deref_mut(&mut self) -> &mut T {
                unsafe { &mut *self.lock.data.get() }
            }
        }

        impl<T> Drop for $name<'_, T> {
            fn drop(&mut self) {
                self.lock.unlock();
            }
        }
    };
}

pub(crate) use lock_guard;
