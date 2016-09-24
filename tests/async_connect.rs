extern crate asyncio;
use std::io;
use std::sync::Arc;
use asyncio::*;
use asyncio::ip::*;
use asyncio::socket_base::*;

static mut goal_flag: bool = false;

fn on_accept1(sv: Arc<TcpListener>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
    if let Ok((_, ep)) = res {
        println!("accepted {}", ep);
        sv.async_accept(bind(on_accept2, &sv));
    } else {
        panic!();
    }
}

fn on_accept2(_: Arc<TcpListener>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
    if let Ok((_, ep)) = res {
        println!("accepted {}", ep);
        unsafe { goal_flag = true; }
    } else {
        panic!();
    }
}

fn on_connect(_: Arc<TcpResolver>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
    if let Ok((_, ep)) = res {
        println!("connected {}", ep);
    } else {
        panic!();
    }
}

#[test]
fn main() {
    let io = &IoService::new();
    let ep = TcpEndpoint::new(IpAddrV4::loopback(), 12345);

    let sv = Arc::new(TcpListener::new(io, Tcp::v4()).unwrap());
    sv.set_option(ReuseAddr::new(true)).unwrap();
    sv.bind(&ep).unwrap();
    sv.listen().unwrap();
    sv.async_accept(bind(on_accept1, &sv));

    let re1 = Arc::new(TcpResolver::new(io));
    re1.async_connect(("localhost", "12345"), bind(on_connect, &re1));
    let re2 = Arc::new(TcpResolver::new(io));
    re2.async_connect(("localhost", "12345"), bind(on_connect, &re2));
    io.run();
    assert!(unsafe { goal_flag });
}
