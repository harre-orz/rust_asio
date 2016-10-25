extern crate asyncio;
use std::io;
use std::sync::Arc;
use asyncio::*;
use asyncio::ip::*;

static mut goal_flag: bool = false;

fn start(io: &IoService) {
    let soc = Arc::new(TcpSocket::new(io, Tcp::v4()).unwrap());
    soc.async_connect(&TcpEndpoint::new(IpAddrV4::loopback(), 12345), wrap(on_connect, &soc));
}

fn on_connect(_: Arc<TcpSocket>, res: io::Result<()>) {
    if let Err(err) = res {
        assert_eq!(err.kind(), io::ErrorKind::ConnectionRefused);
        unsafe { goal_flag = true; }
    } else {
        panic!();
    }
}

#[cfg(target_os = "linux")]
#[test]
fn main() {
    let io = &IoService::new();
    start(io);
    io.run();
    assert!(unsafe { goal_flag })
}
