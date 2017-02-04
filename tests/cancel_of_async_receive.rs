extern crate asyncio;

use std::io;
use std::time::Duration;
use asyncio::*;
use asyncio::ip::*;

static mut GOAL_FLAG: bool = false;

struct UdpClient {
    soc: UdpSocket,
    timer: SteadyTimer,
    buf: [u8; 256],
}

impl UdpClient {
    fn start(ctx: &IoContext) -> io::Result<()> {
        let soc = try!(UdpSocket::new(ctx, Udp::v4()));
        soc.bind(&UdpEndpoint::new(IpAddrV4::loopback(), 12345)).unwrap();
        Ok(IoContext::strand(ctx, UdpClient {
            soc: soc,
            timer: SteadyTimer::new(ctx),
            buf: [0; 256],
        }).dispatch(Self::on_start))
    }

    fn on_start(cl: Strand<Self>) {
        cl.timer.expires_from_now(Duration::new(1, 0));
        cl.timer.async_wait(cl.wrap(Self::on_wait));
        cl.soc.async_receive(&mut cl.get().buf, 0, cl.wrap(Self::on_receive));
    }

    fn on_receive(_: Strand<Self>, res: io::Result<usize>) {
        if let Err(err) = res {
            assert_eq!(err.kind(), io::ErrorKind::Other);  // cancel
            unsafe { GOAL_FLAG = true; }
        } else {
            panic!("{:?}", res);
        }
    }

    fn on_wait(cl: Strand<Self>, res: io::Result<()>) {
        if let Ok(_) = res {
            cl.soc.cancel();
        } else {
            panic!("{:?}", res);
        }
    }
}

#[test]
fn main() {
    let ctx = &IoContext::new().unwrap();
    UdpClient::start(ctx).unwrap();
    ctx.run();
    assert!(unsafe { GOAL_FLAG })
}
