extern crate asyncio;
use std::io;
use std::sync::Arc;
use asyncio::*;
use asyncio::ip::*;
use asyncio::socket_base::*;

static mut GOAL_FLAG: bool = false;

fn on_accept(_: Arc<TcpListener>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
    let _ = res.unwrap();
    println!("on_accept");
}

struct TcpClient {
    soc: TcpSocket,
    buf: Vec<u8>,
}

impl TcpClient {
    fn start(ctx: &IoContext) -> io::Result<()> {
        let mut buf = Vec::with_capacity(1024 * 1024);
        let len = buf.capacity();
        unsafe {
            buf.set_len(len);
        }

        let ep = TcpEndpoint::new(IpAddrV4::loopback(), 12345);
        Ok(
            Strand::new(
                ctx,
                TcpClient {
                    soc: try!(TcpSocket::new(ctx, ep.protocol())),
                    buf: buf,
                },
            ).dispatch(move |st| Self::on_start(st, ep)),
        )
    }

    fn on_start(cl: Strand<Self>, ep: TcpEndpoint) {
        println!("on_dispatch");
        cl.soc.async_connect(&ep, cl.wrap(Self::on_connect));
    }

    fn on_connect(cl: Strand<Self>, res: io::Result<()>) {
        if let Ok(_) = res {
            println!("on_connect");
            cl.soc.async_send(
                cl.buf.as_slice(),
                0,
                cl.wrap(Self::on_send),
            );
        } else {
            panic!("{:?}", res);
        }
    }

    fn on_send(cl: Strand<Self>, res: io::Result<usize>) {
        match res {
            Ok(_) => {
                println!("on_send");
                cl.soc.async_send(
                    cl.buf.as_slice(),
                    0,
                    cl.wrap(Self::on_send),
                );
            }
            Err(err) => {
                match err.kind() {
                    io::ErrorKind::Other |
                    io::ErrorKind::BrokenPipe |
                    io::ErrorKind::ConnectionReset |
                    io::ErrorKind::ConnectionAborted => unsafe {
                        GOAL_FLAG = true;
                    },
                    ec => panic!("{:?}", ec),
                }
            }
        }
    }
}

#[test]
fn main() {
    let ctx = &IoContext::new().unwrap();
    let acc = Arc::new(TcpListener::new(ctx, Tcp::v4()).unwrap());
    acc.set_option(ReuseAddr::new(true)).unwrap();
    acc.bind(&TcpEndpoint::new(IpAddrV4::loopback(), 12345))
        .unwrap();
    acc.listen().unwrap();
    acc.async_accept(wrap(on_accept, &acc));
    TcpClient::start(ctx).unwrap();
    ctx.run();
    assert!(unsafe { GOAL_FLAG });
}
