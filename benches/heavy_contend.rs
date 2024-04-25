use criterion::{criterion_group, criterion_main, Criterion};
use spin_locks::{MSCLock, ThreadLocalMscLock, TicketLock};
use std::sync::Arc;
use std::thread;

const N: usize = 1000000;
const THREAD_N: usize = 8;

fn ticket_lock_test() {
    let lock = Arc::new(TicketLock::new(0));
    let threads: Vec<_> = (0..THREAD_N)
        .map(|_| {
            let lock = lock.clone();
            thread::spawn(move || {
                for _ in 0..N {
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
    assert_eq!(*value, N * THREAD_N);
}

fn msc_lock_test() {
    let lock = Arc::new(MSCLock::new(0));
    let threads: Vec<_> = (0..THREAD_N)
        .map(|_| {
            let lock = lock.clone();
            thread::spawn(move || {
                for _ in 0..N {
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
    assert_eq!(*value, N * THREAD_N);
}

fn thread_local_msc_lock_test() {
    let lock = Arc::new(ThreadLocalMscLock::new(0));
    let threads: Vec<_> = (0..THREAD_N)
        .map(|_| {
            let lock = lock.clone();
            thread::spawn(move || {
                for _ in 0..N {
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
    assert_eq!(*value, N * THREAD_N);
}

// thread_local_msc_lock > msc_lock > ticket_lock
fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("benches");
    group.sample_size(10);
    group.bench_function("ticket_lock", |b| b.iter(ticket_lock_test));
    group.bench_function("msc_lock", |b| b.iter(msc_lock_test));
    group.bench_function("thread_local_msc_lock", |b| {
        b.iter(thread_local_msc_lock_test)
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
