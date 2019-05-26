#![feature(test)]
extern crate asyncio;
extern crate test;

use asyncio::*;
use asyncio::local::*;
use test::Bencher;

#[bench]
fn bench_sync_1000(b: &mut Bencher) {
    let ctx = &IoContext::new().unwrap();
    b.iter(|| {
        let (tx, rx) = connect_pair(ctx, LocalStream).unwrap();
        let mut buf = [0; 1024];
        for _ in 0..1000 {
            tx.send(&buf, 0).unwrap();
            rx.receive(&mut buf, 0).unwrap();
        }
    })
}

#[bench]
fn bench_async01_1000(b: &mut Bencher) {
    let ctx = &IoContext::new().unwrap();
    b.iter(|| {
        ctx.restart();
        spawn(ctx, move |coro| {
            let (tx, rx) = connect_pair(coro.as_ctx(), LocalStream).unwrap();
            let mut buf = [0; 1024];
            for _ in 0..1000 {
                tx.async_send(&buf, 0, coro.wrap()).unwrap();
                rx.async_receive(&mut buf, 0, coro.wrap()).unwrap();
            }
        });
        ctx.run();
    })
}
