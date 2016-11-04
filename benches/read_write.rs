#![feature(test)]
extern crate asyncio;
extern crate test;

use asyncio::*;
use asyncio::local::*;
use test::Bencher;

#[bench]
fn bench_synced_1000(b: &mut Bencher) {
    let io = &IoService::new();
    b.iter(|| {
        let (tx, rx) : (LocalStreamSocket, LocalStreamSocket) = connect_pair(io, LocalStream).unwrap();
        let mut buf = [0; 1024];
        for _ in 0..1000 {
            tx.send(&buf, 0).unwrap();
            rx.receive(&mut buf, 0).unwrap();
        }
    })
}

#[bench]
fn bench_async01_1000(b: &mut Bencher) {
    let io = &IoService::new();
    b.iter(|| {
        io.reset();
        IoService::spawn(io, move |co| {
            let io = co.io_service();
            let (tx, rx) : (LocalStreamSocket, LocalStreamSocket) = connect_pair(io, LocalStream).unwrap();
            let mut buf = [0; 1024];
            for _ in 0..1000 {
                tx.async_send(&buf, 0, co.wrap()).unwrap();
                rx.async_receive(&mut buf, 0, co.wrap()).unwrap();
            }
        });
        io.run();
    })
}

#[bench]
fn bench_async10_1000(b: &mut Bencher) {
    use std::thread;

    let io = &IoService::new();
    b.iter(|| {
        let mut work = Some(IoService::work(io));
        io.reset();

        let mut thrds = Vec::new();
        for _ in 0..10 {
            let io = io.clone();
            thrds.push(thread::spawn(move|| io.run()));
        }
        IoService::spawn(io, move |co| {
            let io = co.io_service();
            let (tx, rx) : (LocalStreamSocket, LocalStreamSocket) = connect_pair(io, LocalStream).unwrap();
            let mut buf = [0; 1024];
            for _ in 0..1000 {
                tx.async_send(&buf, 0, co.wrap()).unwrap();
                rx.async_receive(&mut buf, 0, co.wrap()).unwrap();
            }
            work = None;
        });

        for t in thrds {
            t.join().unwrap();
        }
    })
}
