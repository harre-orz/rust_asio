extern crate asyio;

use asyio::ip::*;
use asyio::socket_base::*;
use asyio::{IoContext, Stream};
use std::time::Duration;

#[test]
fn main() {
    let ctx = &IoContext::new().unwrap();

    // server
    ctx.spawn(|ctx| {
        let ep  = TcpEndpoint::new(IpAddrV4::loopback(), 12345);
        let acc = TcpListener::new(ctx.as_ctx(), ep.protocol()).unwrap();
        acc.set_option(ReuseAddr::YES).unwrap();
        acc.bind(&ep).unwrap();
        acc.listen().unwrap();
        let (soc, ep) = acc.async_accept(ctx).unwrap();

        let mut buf = [0; 8];
        let len = soc.async_read_some(&mut buf, ctx).unwrap();
        assert_eq!(len, 8);

        let len = soc.async_write_some(&buf, ctx).unwrap();
        assert_eq!(len, 8);
    });

    ctx.spawn(|ctx| {
        let ep  = TcpEndpoint::new(IpAddrV4::loopback(), 12345);
        let soc = TcpSocket::new(ctx.as_ctx(), ep.protocol()).unwrap();
        soc.connect(&ep).unwrap();

        let mut buf = [0; 8];
        let len = soc.async_write_some(&buf, ctx).unwrap();
        assert_eq!(len, 8);

        let len = soc.async_read_some(&mut buf, ctx).unwrap();
        assert_eq!(len, 8);
    });

    ctx.run();
}
