#![feature(test)]
extern crate asyncio;
extern crate test;

use asyncio::{IoContext, IoContextWork};
use test::Bencher;

#[bench]
fn bench_thrd01_1000(b: &mut Bencher) {
    let ctx = &IoContext::new().unwrap();
    b.iter(|| {
        ctx.restart();
        let _work = IoContextWork::new(ctx);
        fn repeat(ctx: &IoContext, count: usize) {
            if count > 0 {
                ctx.post(move |ctx| repeat(ctx, count - 1));
            } else {
                ctx.stop();
            }
        }
        repeat(&ctx, 1000);
        ctx.run();
    })
}

#[bench]
fn bench_thrd10_1000(b: &mut Bencher) {
    use std::thread;
    use std::sync::Arc;
    use std::sync::atomic::*;

    static STOP: AtomicBool = ATOMIC_BOOL_INIT;

    let ctx = &IoContext::new().unwrap();
    let mut thrds = Vec::new();
    for _ in 0..4 {
        let ctx = ctx.clone();
        thrds.push(thread::spawn(move || {
            while !STOP.load(Ordering::Relaxed) {
                ctx.run()
            }
        }));
    }

    b.iter(|| {
        ctx.restart();
        let count = Arc::new(AtomicIsize::new(1000));
        fn repeat(ctx: &IoContext, count: Arc<AtomicIsize>) {
            if count.fetch_sub(1, Ordering::SeqCst) <= 1 {
                ctx.stop()
            } else {
                ctx.post(move |ctx: &IoContext| repeat(ctx, count));
            }
        }

        for _ in 0..4 {
            let cnt = count.clone();
            ctx.post(move |ctx: &IoContext| repeat(ctx, cnt));
        }

        ctx.run();
    });

    STOP.store(true, Ordering::SeqCst);
    for thrd in thrds {
        thrd.join().unwrap();
    }
}
