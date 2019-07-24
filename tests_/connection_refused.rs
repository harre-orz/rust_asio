extern crate asyncio;
use std::io;
use std::sync::Arc;
use asyncio::*;
use asyncio::ip::*;

static mut GOAL_FLAG: bool = false;

fn on_connect(_: Arc<TcpSocket>, res: io::Result<()>) {
    if let Err(err) = res {
        assert_eq!(err.kind(), io::ErrorKind::ConnectionRefused);
        unsafe {
            GOAL_FLAG = true;
        }
    } else {
        panic!("{:?}", res);
    }
}

#[test]
fn main() {
    let ctx = &IoContext::new().unwrap();
    let ep = TcpEndpoint::new(IpAddrV4::new(127, 0, 0, 1), 1);
    let soc = Arc::new(TcpSocket::new(ctx, ep.protocol()).unwrap());
    soc.async_connect(&ep, wrap(&soc, on_connect));
    ctx.run();
    assert!(unsafe { GOAL_FLAG })
}
