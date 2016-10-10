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
    fn start(io: &IoService, soc: TcpSocket) {
        let daytime = Strand::new(io, DaytimeTcp {
            soc: soc,
            buf: format!("{}\r\n", time::now().ctime())
        });

        daytime.soc.async_write_some(daytime.buf.as_bytes(), daytime.wrap(Self::on_send));
    }

    fn on_send(_: Strand<Self>, _: io::Result<usize>) {
    }
}

fn on_accept(sv: Strand<TcpListener>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
    if let Ok((soc, ep)) = res {
        println!("connected from {:?}", ep);

        DaytimeTcp::start(sv.io_service(), soc);

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
            daytime.soc.async_receive_from(unsafe { &mut daytime.get().buf }, 0, daytime.wrap(Self::on_receive));
        }
    }
}

fn main() {
    let io = &IoService::new();

    // TCP
    let sv = Strand::new(io, TcpListener::new(io, Tcp::v4()).unwrap());
    sv.set_option(ReuseAddr::new(true)).unwrap();
    sv.bind(&TcpEndpoint::new(IpAddrV4::any(), 13)).unwrap();
    sv.listen().unwrap();
    sv.async_accept(sv.wrap(on_accept));

    // UDP
    let daytime = Strand::new(io, DaytimeUdp {
        soc: UdpSocket::new(io, Udp::v4()).unwrap(),
        buf: [0; 128],
    });
    daytime.soc.set_option(ReuseAddr::new(true)).unwrap();
    daytime.soc.bind(&UdpEndpoint::new(IpAddrV4::any(), 13)).unwrap();
    daytime.soc.async_receive_from(unsafe { &mut daytime.get().buf }, 0, daytime.wrap(DaytimeUdp::on_receive));

    io.run();
}
