extern crate asyncio;
use std::io;
use std::time::Duration;
use asyncio::*;
use asyncio::ip::*;
use asyncio::socket_base::*;

static mut GOAL_FLAG: bool = false;

struct TcpAcceptor {
    soc: TcpListener,
    timer: SteadyTimer,
}

impl TcpAcceptor {
    fn start(ctx: &IoContext) -> io::Result<()> {
        let ep = TcpEndpoint::new(IpAddrV4::loopback(), 12345);
        let soc = try!(TcpListener::new(ctx, ep.protocol()));
        let _ = try!(soc.set_option(ReuseAddr::new(true)));
        let _ = try!(soc.bind(&ep));
        let _ = try!(soc.listen());
        Ok(
            Strand::new(
                ctx,
                TcpAcceptor {
                    soc: soc,
                    timer: SteadyTimer::new(ctx),
                },
            ).dispatch(Self::on_start),
        )
    }

    fn on_start(mut acc: Strand<Self>) {
        acc.soc.async_accept(acc.wrap(Self::on_accept));
        acc.timer.expires_from_now(Duration::new(1, 0));
        acc.timer.async_wait(acc.wrap(Self::on_wait));
    }

    fn on_accept(_: Strand<Self>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
        if let Err(err) = res {
            println!("on_accept");
            assert_eq!(err.kind(), io::ErrorKind::Other); // cancel
            unsafe {
                GOAL_FLAG = true;
            }
        } else {
            panic!("{:?}", res);
        }
    }

    fn on_wait(mut acc: Strand<Self>, res: io::Result<()>) {
        if let Ok(_) = res {
            println!("on_wait");
            acc.soc.cancel();
        } else {
            panic!("{:?}", res);
        }
    }
}

#[test]
fn main() {
    let ctx = &IoContext::new().unwrap();
    TcpAcceptor::start(ctx).unwrap();
    ctx.run();
    assert!(unsafe { GOAL_FLAG });
}
