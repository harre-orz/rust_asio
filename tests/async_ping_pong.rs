extern crate asyio;

use asyio::ip::*;
use asyio::{AsIoContext, IoContext};
use std::time::Duration;

#[test]
fn main() {
    let ctx = &IoContext::new().unwrap();
    ctx.spawn(|ctx| {
        let ep = UdpEndpoint::new(IpAddrV4::loopback(), 12345);
        println!("ep = {:?}", ep);
        let mut soc = UdpSocket::new(ctx.as_ctx(), Udp::v4()).unwrap();
        //let len = soc.async_send_to(&[10; 8], 0, &ep, ctx).unwrap();
        let mut buf = [10; 8];
        soc.set_timeout(Duration::from_secs(3));
        let len = soc.async_receive(&mut buf, 0, ctx).unwrap();
        assert_eq!(len, 8);
    });
    ctx.run();
}
