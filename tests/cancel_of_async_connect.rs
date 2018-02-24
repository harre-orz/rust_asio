extern crate asyncio;
use std::io;
use std::time::Duration;
use asyncio::*;
use asyncio::ip::*;

static mut GOAL_FLAG: bool = false;

struct TcpClient {
    soc: TcpSocket,
    timer: SteadyTimer,
}

impl TcpClient {
    fn start(ctx: &IoContext) -> io::Result<()> {
        let ep = TcpEndpoint::new(IpAddrV4::new(1, 2, 3, 4), 12345);
        Ok(
            Strand::new(
                ctx,
                TcpClient {
                    soc: try!(TcpSocket::new(ctx, ep.protocol())),
                    timer: SteadyTimer::new(ctx),
                },
            ).dispatch(move |cl| Self::on_start(cl, ep)),
        )
    }

    fn on_start(mut cl: Strand<Self>, ep: TcpEndpoint) {
        cl.timer.expires_from_now(Duration::new(1, 0));
        cl.timer.async_wait(cl.wrap(Self::on_wait));
        cl.soc.async_connect(&ep, cl.wrap(Self::on_connect));
    }

    fn on_connect(_: Strand<Self>, res: io::Result<()>) {
        if let Err(err) = res {
            println!("on_connect");
            assert_eq!(err.kind(), io::ErrorKind::Other); // Cancel
            unsafe {
                GOAL_FLAG = true;
            }
        } else {
            panic!("{:?}", res);
        }
    }

    fn on_wait(mut cl: Strand<Self>, res: io::Result<()>) {
        if let Ok(_) = res {
            println!("on_wait");
            cl.soc.cancel();
        } else {
            panic!("{:?}", res);
        }
    }
}

#[test]
fn main() {
    let ctx = &IoContext::new().unwrap();
    TcpClient::start(ctx).unwrap();
    ctx.run();
    assert!(unsafe { GOAL_FLAG })
}
