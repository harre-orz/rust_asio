extern crate asio;

use std::env;
use std::str;
use asio::*;
use asio::ip::*;

fn main() {
    // All programs that use asio need to have at least one io_service object.
    let io = &IoService::new();

    // Makes a TcpResolver.
    let resolver = TcpResolver::new(io);

    let host = &env::args().nth(1).unwrap()[..];

    // Returns connected TcpSocket with TcpEndpoint.
    let (soc, ep) = resolver.connect((host, "daytime")).unwrap();
    println!("connected to {}", ep);

    // The TcpSocket read message from the TCP server.
    let mut buf = [0; 256];
    soc.read_some(&mut buf).unwrap();

    println!("{}", str::from_utf8(&buf).unwrap());
}
