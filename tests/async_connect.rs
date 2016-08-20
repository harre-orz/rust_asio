extern crate asio;
use std::io;
use std::sync::Arc;
use asio::*;
use asio::ip::*;
use asio::socket_base::*;

static mut goal_flag: bool = false;

fn on_accept1(sv: Arc<TcpListener>, res: io::Result<(TcpSocket, TcpEndpoint)>, _: &IoService) {
    if let Ok((_, ep)) = res {
        println!("accepted {}", ep);
        sv.async_accept(bind(on_accept2, &sv));
    } else {
        panic!();
    }
}

fn on_accept2(_: Arc<TcpListener>, res: io::Result<(TcpSocket, TcpEndpoint)>, _: &IoService) {
    if let Ok((_, ep)) = res {
        println!("accepted {}", ep);
        unsafe { goal_flag = true; }
    } else {
        panic!();
    }
}
fn on_connect(res: io::Result<(TcpSocket, TcpEndpoint)>) {
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

    let re = TcpResolver::new(io);
    async_connect(io, re.resolve(("localhost", "12345")).unwrap(), on_connect);
    async_connect(io, Some(ep).into_iter(), on_connect);
    io.run();
    assert!(unsafe { goal_flag });
}
