extern crate asyncio;
extern crate time;

use asyncio::*;
use asyncio::ip::*;
use asyncio::socket_base::*;

fn main() {
    let ctx = &IoContext::new().unwrap();

    let soc = UdpSocket::new(ctx, Udp::v4()).unwrap();

    soc.set_option(ReuseAddr::new(true)).unwrap();

    soc.bind(&UdpEndpoint::new(IpAddrV4::any(), 13)).unwrap();

    let mut buf = [0; 128];
    while let Ok((_, ep)) = soc.receive_from(&mut buf, 0) {
        println!("receive from {}", ep);

        let buf = format!("{}\r\n", time::now().ctime());
        soc.send_to(buf.as_bytes(), 0, ep).unwrap();
    }
}
