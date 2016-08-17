extern crate asio;

use std::env::args;
use std::process::exit;
use std::str;
use asio::*;
use asio::ip::*;

fn main() {
    let host = args().nth(1).unwrap_or_else(|| {
        println!("usage: client <host>");
        exit(1);
    });

    // All programs that use asio need to have at least one io_service object.
    let io = &IoService::new();

    // Makes a resolving object.
    let res = TcpResolver::new(io);

    // Returns connected TcpSocket with TcpEndpoint.
    let it = res.resolve((&host[..], "daytime")).unwrap();
    let (soc, ep) = connect(io, it).unwrap();
    let soc: TcpSocket = soc;
    println!("connected to {}", ep);

    // The TcpSocket read message from the TCP server.
    let mut buf = [0; 256];
    let len = soc.read_some(&mut buf).unwrap();

    println!("{}", str::from_utf8(&buf[..len]).unwrap());
}
