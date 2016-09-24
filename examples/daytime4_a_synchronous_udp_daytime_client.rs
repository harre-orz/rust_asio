extern crate asyncio;

use std::env::args;
use std::process::exit;
use std::str;
use asyncio::*;
use asyncio::ip::*;

fn main() {
    let host = args().nth(1).unwrap_or_else(|| {
        println!("usage: client <host>");
        exit(1);
    });

    let io = &IoService::new();

    let ep = UdpResolver::new(io).resolve((Udp::v4(), host, "daytime")).unwrap().next().unwrap();

    let soc = UdpSocket::new(io, ep.protocol()).unwrap();

    let send_buf = [0];
    soc.send_to(&send_buf, 0, ep).unwrap();

    let mut recv_buf = [0; 128];
    let (len, ep) = soc.receive_from(&mut recv_buf, 0).unwrap();
    println!("receive from {}", ep);

    println!("{}", str::from_utf8(&recv_buf[..len]).unwrap());
}
