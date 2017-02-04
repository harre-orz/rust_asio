extern crate asyncio;
extern crate time;

use std::io;
use asyncio::*;
use asyncio::ip::*;
use asyncio::socket_base::*;

struct DaytimeTcp {
    soc: TcpSocket,
    buf: String,
}

impl DaytimeTcp {
    fn start(ctx: &IoContext, soc: TcpSocket) {
        let daytime = IoContext::strand(ctx, DaytimeTcp {
            soc: soc,
            buf: format!("{}\r\n", time::now().ctime())
        });
        daytime.dispatch(Self::on_start);
    }

    fn on_start(daytime: Strand<Self>) {
        daytime.soc.async_write_some(daytime.buf.as_bytes(), daytime.wrap(Self::on_send));
    }

    fn on_send(_: Strand<Self>, _: io::Result<usize>) {
    }
}

fn on_start(sv: Strand<TcpListener>) {
    sv.set_option(ReuseAddr::new(true)).unwrap();
    sv.bind(&TcpEndpoint::new(IpAddrV4::any(), 13)).unwrap();
    sv.listen().unwrap();
    sv.async_accept(sv.wrap(on_accept));
}

fn on_accept(sv: Strand<TcpListener>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
    if let Ok((soc, ep)) = res {
        println!("connected from {:?}", ep);

        DaytimeTcp::start(sv.as_ctx(), soc);

        sv.async_accept(sv.wrap(on_accept));
    }
}

struct DaytimeUdp {
    soc: UdpSocket,
    buf: [u8; 128],
}

impl DaytimeUdp {
    fn on_receive(mut daytime: Strand<Self>, res: io::Result<(usize, UdpEndpoint)>) {
        if let Ok((_, ep)) = res {
            println!("receive from {}", ep);

            let buf = format!("{}\r\n", time::now().ctime());
            let len = buf.len();
            daytime.buf[..len].copy_from_slice(buf.as_bytes());
            daytime.soc.async_send_to(&daytime.buf[..len], 0, ep, daytime.wrap(Self::on_send));
        }
    }

    fn on_send(daytime: Strand<Self>, res: io::Result<usize>) {
        if let Ok(_) = res {
            daytime.soc.async_receive_from(&mut daytime.get().buf, 0, daytime.wrap(Self::on_receive));
        }
    }
}

fn main() {
    let ctx = &IoContext::new().unwrap();

    // TCP
    let tcp = IoContext::strand(ctx, TcpListener::new(ctx, Tcp::v4()).unwrap());
    tcp.dispatch(on_start);

    // UDP
    let udp = IoContext::strand(ctx, DaytimeUdp {
        soc: UdpSocket::new(ctx, Udp::v4()).unwrap(),
        buf: [0; 128],
    });
    udp.dispatch(|daytime| {
        daytime.soc.set_option(ReuseAddr::new(true)).unwrap();
        daytime.soc.bind(&UdpEndpoint::new(IpAddrV4::any(), 13)).unwrap();
        daytime.soc.async_receive_from(&mut daytime.get().buf, 0, daytime.wrap(DaytimeUdp::on_receive));
    });

    ctx.run();
}
