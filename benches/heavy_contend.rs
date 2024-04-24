use criterion::{criterion_group, criterion_main, Criterion};
use spin_locks::{MSCLock, TicketLock};
use std::sync::Arc;
use std::thread;

fn ticket_lock_test() {
    let n = 1000000;
    let thread_n = 8;
    let lock = Arc::new(TicketLock::new(0));
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

fn msc_lock_test() {
    let n = 1000000;
    let thread_n = 8;
    let lock = Arc::new(MSCLock::new(0));
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

// msc_lock does run faster
fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("benches");
    group.sample_size(10);
    group.bench_function("ticket_lock", |b| b.iter(ticket_lock_test));
    group.bench_function("msc_lock", |b| b.iter(msc_lock_test));
}

criterion_group!(benches, bench);
criterion_main!(benches);
