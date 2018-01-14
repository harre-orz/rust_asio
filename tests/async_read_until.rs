extern crate asyncio;
use std::io;
use std::sync::Arc;
use asyncio::*;
use asyncio::ip::*;
use asyncio::socket_base::*;

static mut GOAL_FLAG: bool = false;

fn start(ctx: &IoContext) {
    let acc = Arc::new(TcpListener::new(ctx, Tcp::v4()).unwrap());
    acc.set_option(ReuseAddr::new(true)).unwrap();
    acc.bind(&TcpEndpoint::new(IpAddrV4::new(127,0,0,1), 12345)).unwrap();
    acc.listen().unwrap();
    acc.async_accept(wrap(on_accept, &acc));
}

fn on_accept(_: Arc<TcpListener>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
    let (soc, _) = res.unwrap();
    TcpServer::start(soc);
}

struct TcpServer {
    soc: TcpSocket,
    buf: [u8; 1],
}

impl TcpServer {
    fn start(soc: TcpSocket) {
        let mut vec = Vec::new();
        vec.push('\r' as u8);
        vec.push('\n' as u8);
        for _ in 0..10000 {
            vec.push(0x30);
        }
        vec.push('\r' as u8);
        vec.push('\n' as u8);
        soc.write_some(vec.as_slice()).unwrap();

        let ctx = &soc.as_ctx().clone();
        let sv = Strand::new(ctx, TcpServer {
            soc: soc,
            buf: [0],
        });
        sv.dispatch(Self::on_start);
    }

    fn on_start(sv: Strand<Self>) {
        sv.soc.async_read_some(&mut sv.get().buf, sv.wrap(Self::on_read));
    }

    fn on_read(_: Strand<Self>, res: io::Result<usize>) {
        assert!(res.is_err());
    }
}

struct TcpClient {
    soc: TcpSocket,
    buf: StreamBuf,
}

impl TcpClient {
    fn start(ctx: &IoContext) {
        let cl = Strand::new(ctx, TcpClient {
            soc: TcpSocket::new(ctx, Tcp::v4()).unwrap(),
            buf: StreamBuf::new(),
        });
        cl.dispatch(Self::on_start);
    }

    fn on_start(cl: Strand<Self>) {
        cl.soc.connect(&TcpEndpoint::new(IpAddrV4::new(127,0,0,1), 12345)).unwrap();
        cl.soc.async_read_until(&mut cl.get().buf, "\r\n", cl.wrap(Self::on_read1));
    }

    fn on_read1(cl: Strand<Self>, res: io::Result<usize>) {
        let size = res.unwrap();
        assert_eq!(size, 2);
        cl.soc.async_read_until(&mut cl.get().buf, "\r\n", cl.wrap(Self::on_read2));
    }

    fn on_read2(mut cl: Strand<Self>, res: io::Result<usize>) {
        let size = res.unwrap();
        assert_eq!(size, 2);
        cl.buf.consume(2);
        cl.soc.async_read_until(&mut cl.get().buf, "\r\n", cl.wrap(Self::on_read3));
    }

    fn on_read3(_: Strand<Self>, res: io::Result<usize>) {
        let size = res.unwrap();
        assert_eq!(size, 10002);
        unsafe { GOAL_FLAG = true; }
    }
}

#[test]
fn main() {
    let ctx = &IoContext::new().unwrap();
    start(ctx);
    TcpClient::start(ctx);
    ctx.run();
    assert!(unsafe { GOAL_FLAG });
}
