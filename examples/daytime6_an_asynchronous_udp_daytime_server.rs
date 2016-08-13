extern crate asio;
extern crate time;

use std::io;
use asio::*;
use asio::ip::*;
use asio::socket_base::*;

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

    let daytime = Strand::new(io, DaytimeUdp {
        soc: UdpSocket::new(io, Udp::v4()).unwrap(),
        buf: [0; 128],
    });

    daytime.soc.set_option(ReuseAddr::new(true)).unwrap();

    daytime.soc.bind(&UdpEndpoint::new(IpAddrV4::any(), 13)).unwrap();

    daytime.soc.async_receive_from(unsafe { &mut daytime.get().buf }, 0, daytime.wrap(DaytimeUdp::on_receive));

    io.run();
}
