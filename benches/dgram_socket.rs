#![feature(test)]
extern crate asyncio;
extern crate test;

use asyncio::*;
use asyncio::ip::*;
use asyncio::socket_base::ReuseAddr;
use test::Bencher;

#[bench]
fn bench_single_sync_100(b: &mut Bencher) {
    let ctx = &IoContext::new().unwrap();
    let ep = UdpEndpoint::new(Udp::v4(), 12345);
    let sv = UdpSocket::new(ctx, ep.protocol()).unwrap();
    let cl = UdpSocket::new(ctx, ep.protocol()).unwrap();
    sv.set_option(ReuseAddr::new(true)).unwrap();
    sv.bind(&ep).unwrap();
    cl.connect(&ep).unwrap();

    let mut buf = [0; 1024];
    b.iter(|| {
        for _ in 0..100 {
            cl.send(&buf, 0).unwrap();
            sv.receive(&mut buf, 0).unwrap();
        }
    })
}

struct S {
    sv: UdpSocket,
    cl: UdpSocket,
    buf: [u8; 1024],
}

#[bench]
fn bench_single_async_100(b: &mut Bencher) {
    let ctx = &IoContext::new().unwrap();
    let ep = UdpEndpoint::new(Udp::v4(), 12345);
    let sv = UdpSocket::new(ctx, ep.protocol()).unwrap();
    let cl = UdpSocket::new(ctx, ep.protocol()).unwrap();
    sv.set_option(ReuseAddr::new(true)).unwrap();
    sv.bind(&ep).unwrap();
    cl.connect(&ep).unwrap();
    let s = IoContext::strand(
        ctx,
        S {
            sv: sv,
            cl: cl,
            buf: [0; 1024],
        },
    );
    b.iter(|| {
        ctx.restart();
        for _ in 0..100 {
            s.dispatch(move |s| {
                s.cl.async_send(&s.buf, 0, s.wrap(move |_, _| {}));
                s.sv
                    .async_receive(&mut s.get().buf, 0, s.wrap(move |_, _| {}));
            });
        }
        ctx.run();
    })
}
