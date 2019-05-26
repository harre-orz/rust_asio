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
