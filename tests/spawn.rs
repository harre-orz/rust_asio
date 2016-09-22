extern crate asio;
use std::io;
use std::sync::Arc;
use asio::*;
use asio::ip::*;
use asio::socket_base::*;


fn on_accept(io: Arc<TcpListener>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
    let (soc, _) = res.unwrap();
    spawn(io.io_service(), move |coro| {
        let size = soc.async_write_some(&"hello".as_bytes(), coro.yield_with()).unwrap();
        assert_eq!(size, 5);

        let mut buf = [0; 256];
        let size = soc.async_read_some(&mut buf, coro.yield_with()).unwrap();
        assert_eq!(&buf[..size], "world".as_bytes());
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
    soc.async_accept(bind(on_accept, &soc));

    spawn(io, move |coro| {
        let soc = TcpSocket::new(coro, Tcp::v4()).unwrap();
        soc.async_connect(&ep, coro.yield_with()).unwrap();

        let mut buf = [0; 256];
        let size = soc.async_read_some(&mut buf, coro.yield_with()).unwrap();
        assert_eq!(&buf[..size], "hello".as_bytes());

        let size = soc.async_write_some(&"world".as_bytes(), coro.yield_with()).unwrap();
        assert_eq!(size, 5);
    });

    io.run();
}
