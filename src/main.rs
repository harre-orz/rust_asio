extern crate net;
use net::*;
use std::str::FromStr;

fn main() {
    let io = net::IoService::default();

    let s = net::ip::IcmpSocket::bind(&io, &net::ip::Endpoint::new((net::ip::IpAddrV6::default(), 0)));
    match s {
        Ok(mut soc) => {
            println!("icmp bound");
            let mut buf = [0; 100];
            match soc.receive(&mut buf[..]) {
                Ok(size) => {
                    println!("{}", size);
                },
                Err(msg) => {
                    println!("2 {}", msg);
                },
            }
        },
        Err(msg) => {
            println!("{}", msg)
        },
    }

    let addr = net::ip::IpAddrV4::from_str("1.2.3.4").unwrap();
    println!("{}", addr);
    let ep: net::ip::Endpoint<net::ip::Tcp> = net::ip::Endpoint::new((addr, 12345));
    println!("{:?}", ep);
    match net::ip::TcpStream::connect(&io, &ep) {
        Ok(mut soc) => {
            let mut buf = [0; 100];
            match soc.receive(&mut buf[..]) {
                Ok(size) => {
                    println!("{}", size);
                },
                Err(msg) => {
                    println!("2 {}", msg);
                },
            }
        },
        Err(msg) => {
            println!("1 {}", msg)
        }
    };

    let ep = net::local::Endpoint::new("sample.sock");
    match net::local::LocalStream::connect(&io, &ep) {
        Ok(soc) => (),
        _ => (),
    };
}
