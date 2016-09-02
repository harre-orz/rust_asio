extern crate asio;
use std::io;
use std::sync::Arc;
use asio::*;
use asio::ip::*;
use asio::socket_base::*;

static mut goal_flag: bool = false;

fn on_accept(sv: Arc<TcpListener>, res: io::Result<(TcpSocket, TcpEndpoint)>, _: &IoService) {
    if let Ok((_, ep)) = res {
        println!("accepted {}", ep);
        unsafe { goal_flag = true; }
    } else {
        panic!();
    }
}

fn on_connect(_: Arc<TcpResolver>, res: io::Result<(TcpSocket, TcpEndpoint)>, _: &IoService) {
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
    sv.async_accept(bind(on_accept, &sv));

    let re = Arc::new(TcpResolver::new(io));
    re.async_connect(("localhost", "12345"), bind(on_connect, &re));
    io.run();
    assert!(unsafe { goal_flag });
}
