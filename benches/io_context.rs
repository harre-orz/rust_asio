#![feature(test)]
extern crate asyncio;
extern crate test;

use asyncio::IoContext;
use test::Bencher;

#[bench]
fn bench_thrd01_1000(b: &mut Bencher) {
    let ctx = &IoContext::new().unwrap();
    b.iter(|| {
        ctx.restart();
        let _work = IoContext::work(ctx);
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

    let ctx = &IoContext::new().unwrap();
    b.iter(|| {
        let _work = IoContext::work(ctx);
        ctx.restart();

        let count = Arc::new(AtomicIsize::new(1000));
        let mut thrds = Vec::new();
        for _ in 0..10 {
            let ctx = ctx.clone();
            let count = count.clone();
            thrds.push(thread::spawn(move || {
                fn repeat(ctx: &IoContext, count: Arc<AtomicIsize>) {
                    match count.fetch_sub(1, Ordering::SeqCst) {
                        1 => ctx.stop(),
                        n if n > 1 => ctx.post(move |ctx| repeat(ctx, count)),
                        _ => (),
                    }
                }
                repeat(&ctx, count);
                ctx.run()
            }));
        }

        for thrd in thrds {
            thrd.join().unwrap();
        }
    });
}
