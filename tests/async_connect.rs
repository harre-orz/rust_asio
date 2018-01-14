extern crate asyncio;
use std::io;
use std::sync::{Arc, Mutex};
use asyncio::*;
use asyncio::ip::*;
use asyncio::socket_base::*;

static mut GOAL_FLAG: bool = false;

fn on_accept1(sv: Arc<Mutex<TcpListener>>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
    if let Ok((_, ep)) = res {
        println!("accepted {}", ep);
        sv.lock().unwrap().async_accept(wrap(on_accept2, &sv));
    } else {
        panic!("{:?}", res);
    }
}

fn on_accept2(_: Arc<Mutex<TcpListener>>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
    if let Ok((_, ep)) = res {
        println!("accepted {}", ep);
        unsafe {
            GOAL_FLAG = true;
        }
    } else {
        panic!("{:?}", res);
    }
}

fn on_connect(_: Arc<Mutex<TcpResolver>>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
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

    let sv = TcpListener::new(ctx, Tcp::v4()).unwrap();
    sv.set_option(ReuseAddr::new(true)).unwrap();
    sv.bind(&ep).unwrap();
    sv.listen().unwrap();
    let sv = Arc::new(Mutex::new(sv));
    sv.lock().unwrap().async_accept(wrap(on_accept1, &sv));

    let re1 = Arc::new(Mutex::new(TcpResolver::new(ctx)));
    re1.lock()
        .unwrap()
        .async_connect(("127.0.0.1", "12345"), wrap(on_connect, &re1));
    let re2 = Arc::new(Mutex::new(TcpResolver::new(ctx)));
    re2.lock()
        .unwrap()
        .async_connect(("127.0.0.1", "12345"), wrap(on_connect, &re2));
    ctx.run();
    assert!(unsafe { GOAL_FLAG });
}
