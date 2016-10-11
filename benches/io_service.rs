#![feature(test)]
extern crate asyncio;
extern crate test;

use asyncio::IoService;
use test::Bencher;

#[bench]
fn bench_thrd01_1000(b: &mut Bencher) {
    let io = &IoService::new();
    b.iter(|| {
        io.reset();
        let _work = IoService::work(io);
        fn repeat(io: &IoService, count: usize) {
            if count > 0 {
                io.post(move |io| repeat(io, count-1));
            } else {
                io.stop();
            }
        }
        repeat(&io, 1000);
        io.run();
    })
}

#[bench]
fn bench_thrd10_1000(b: &mut Bencher) {
    use std::thread;
    use std::sync::Arc;
    use std::sync::atomic::*;

    let io = &IoService::new();
    b.iter(|| {
        let _work = IoService::work(io);
        io.reset();

        let count = Arc::new(AtomicIsize::new(1000));
        let mut thrds = Vec::new();
        for _ in 0..4 {
            let io = io.clone();
            let count = count.clone();
            thrds.push(thread::spawn(move || {
                fn repeat(io: &IoService, count: Arc<AtomicIsize>) {
                    match count.fetch_sub(1, Ordering::SeqCst) {
                        1 => io.stop(),
                        n if n > 1 => io.post(move |io| repeat(io, count)),
                        _ => (),
                    }
                }
                repeat(&io, count);
                io.run()
            }));
        }

        for thrd in thrds {
            thrd.join().unwrap();
        }
    });
}
