#![feature(test)]
extern crate asio;
extern crate test;

use asio::IoService;

use std::thread;
use std::sync::Arc;
use std::sync::atomic::*;
use test::Bencher;

#[bench]
fn bench_thrd01_1000(b: &mut Bencher) {
    let io = IoService::new();
    b.iter(|| {
        fn repeat(io: &IoService, count: usize) {
            if count > 0 {
                io.post(move |io| repeat(io, count-1));
            }
        }
        repeat(&io, 1000);
        io.run();
    })
}

#[bench]
fn bench_thrd10_1000(b: &mut Bencher) {
    static STOP_BENCH: AtomicBool = ATOMIC_BOOL_INIT;
    let io = IoService::new();
    let mut thrd = Vec::new();
    for _ in 0..10 {
        let ios = io.clone();
        thrd.push(thread::spawn(move || while !STOP_BENCH.load(Ordering::Relaxed) {
            ios.run();
            thread::yield_now();
        }));
    }
    b.iter(|| {
        io.work(|io| {
            let count = Arc::new(AtomicUsize::new(1000));
            for _ in 0..count.load(Ordering::Relaxed) {
                let count = count.clone();
                io.post(move |io| {
                    if count.fetch_sub(1, Ordering::SeqCst) == 1 {
                        io.stop();
                    }
                });
            }
        });
    });

    STOP_BENCH.store(true, Ordering::Relaxed);
    for th in thrd {
        th.join().unwrap();
    }
}
