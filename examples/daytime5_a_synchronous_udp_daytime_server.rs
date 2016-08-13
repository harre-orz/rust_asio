extern crate asio;
extern crate time;

use asio::*;
use asio::ip::*;
use asio::socket_base::*;

fn main() {
    let io = &IoService::new();

    let soc = UdpSocket::new(io, Udp::v4()).unwrap();

    soc.set_option(ReuseAddr::new(true)).unwrap();

    soc.bind(&UdpEndpoint::new(IpAddrV4::any(), 13)).unwrap();

    let mut buf = [0; 128];
    while let Ok((_, ep)) = soc.receive_from(&mut buf, 0) {
        println!("receive from {}", ep);

        let buf = format!("{}\r\n", time::now().ctime());
        soc.send_to(buf.as_bytes(), 0, ep).unwrap();
    }
}
