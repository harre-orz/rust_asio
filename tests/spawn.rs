extern crate asyncio;

use std::io;
use std::sync::Arc;
use asyncio::*;
use asyncio::ip::*;
use asyncio::socket_base::*;

fn on_accept(acc: Arc<TcpListener>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
    let (soc, _) = res.unwrap();
    spawn(acc.as_ctx(), move |coro| {
        println!("sv accepted");

        let len = soc.async_write_some(&"hello".as_bytes(), coro.wrap())
            .unwrap();
        println!("sv written {}", len);
        assert_eq!(len, 5);

        let mut buf = [0; 256];
        let len = soc.async_read_some(&mut buf, coro.wrap()).unwrap();
        println!("sv readed {}", len);
        assert_eq!(&buf[..len], "world".as_bytes());
    }).unwrap();
}

#[test]
fn main() {
    let ctx = &IoContext::new().unwrap();
    let ep = TcpEndpoint::new(IpAddrV4::loopback(), 12345);

    let soc = Arc::new(TcpListener::new(ctx, Tcp::v4()).unwrap());
    soc.set_option(ReuseAddr::new(true)).unwrap();
    soc.bind(&ep).unwrap();
    soc.listen().unwrap();
    soc.async_accept(wrap(&soc, on_accept));

    spawn(ctx, move |coro| {
        let soc = TcpSocket::new(coro.as_ctx(), Tcp::v4()).unwrap();
        soc.async_connect(&ep, coro.wrap()).unwrap();
        println!("cl connected");

        let mut buf = [0; 256];
        let len = soc.async_read_some(&mut buf, coro.wrap()).unwrap();
        println!("cl readed {}", len);
        assert_eq!(&buf[..len], "hello".as_bytes());

        let len = soc.async_write_some(&"world".as_bytes(), coro.wrap())
            .unwrap();
        println!("cl written {}", len);
        assert_eq!(len, 5);
    }).unwrap();

    ctx.run();
}
