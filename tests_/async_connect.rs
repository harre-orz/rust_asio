extern crate asyncio;
use std::io;
use std::sync::Arc;
use asyncio::*;
use asyncio::ip::*;
use asyncio::socket_base::*;

static mut GOAL_FLAG: bool = false;

fn on_accept1(sv: Arc<TcpListener>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
    if let Ok((_, ep)) = res {
        println!("accepted {}", ep);
        sv.async_accept(wrap(&sv, on_accept2));
    } else {
        panic!("{:?}", res);
    }
}

fn on_accept2(_: Arc<TcpListener>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
    if let Ok((_, ep)) = res {
        println!("accepted {}", ep);
        unsafe {
            GOAL_FLAG = true;
        }
    } else {
        panic!("{:?}", res);
    }
}

fn on_connect(_: Arc<TcpResolver>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
    if let Ok((_, ep)) = res {
        println!("connected {}", ep);
    } else {
        panic!("{:?}", res);
    }
}

#[test]
fn main() {
    let ctx = &IoContext::new().unwrap();
    let ep = TcpEndpoint::new(IpAddrV4::loopback(), 12345);

    let sv = Arc::new(TcpListener::new(ctx, Tcp::v4()).unwrap());
    sv.set_option(ReuseAddr::new(true)).unwrap();
    sv.bind(&ep).unwrap();
    sv.listen().unwrap();
    sv.async_accept(wrap(&sv, on_accept1));

    let re1 = Arc::new(TcpResolver::new(ctx));
    re1.async_connect(("127.0.0.1", "12345"), wrap(&re1, on_connect));
    let re2 = Arc::new(TcpResolver::new(ctx));
    re2.async_connect(("127.0.0.1", "12345"), wrap(&re2, on_connect));
    ctx.run();
    assert!(unsafe { GOAL_FLAG });
}
