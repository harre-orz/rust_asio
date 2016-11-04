extern crate asyncio;

use std::io;
use std::sync::Arc;
use asyncio::*;
use asyncio::ip::*;
use asyncio::socket_base::*;

fn on_accept(io: Arc<TcpListener>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
    let (soc, _) = res.unwrap();
    IoService::spawn(io.io_service(), move |coro| {
        println!("sv accepted");

        let len = soc.async_write_some(&"hello".as_bytes(), coro.wrap()).unwrap();
        println!("sv written {}", len);
        assert_eq!(len, 5);

        let mut buf = [0; 256];
        let len = soc.async_read_some(&mut buf, coro.wrap()).unwrap();
        println!("sv readed {}", len);
        assert_eq!(&buf[..len], "world".as_bytes());
    });
}

#[test]
fn main() {
    let io = &IoService::new();
    let ep = TcpEndpoint::new(IpAddrV4::loopback(), 12345);

    let soc = Arc::new(TcpListener::new(io, Tcp::v4()).unwrap());
    soc.set_option(ReuseAddr::new(true)).unwrap();
    soc.bind(&ep).unwrap();
    soc.listen().unwrap();
    soc.async_accept(wrap(on_accept, &soc));

    IoService::spawn(io, move |coro| {
        let soc = TcpSocket::new(coro.io_service(), Tcp::v4()).unwrap();
        soc.async_connect(&ep, coro.wrap()).unwrap();
        println!("cl connected");

        let mut buf = [0; 256];
        let len = soc.async_read_some(&mut buf, coro.wrap()).unwrap();
        println!("cl readed {}", len);
        assert_eq!(&buf[..len], "hello".as_bytes());

        let len = soc.async_write_some(&"world".as_bytes(), coro.wrap()).unwrap();
        println!("cl written {}", len);
        assert_eq!(len, 5);
    });

    io.run();
}
