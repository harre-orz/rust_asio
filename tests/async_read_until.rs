extern crate asio;
use std::io;
use asio::*;
use asio::ip::*;
use asio::socket_base::*;

static mut goal_flag: bool = false;

struct TcpAcceptor {
    soc: TcpListener,
}

impl TcpAcceptor {
    fn start(io: &IoService) {
        let acc = Strand::new(io, TcpAcceptor {
            soc: TcpListener::new(io, Tcp::v4()).unwrap(),
        });
        acc.soc.set_option(ReuseAddr::new(true)).unwrap();
        acc.soc.bind(&TcpEndpoint::new(IpAddrV4::new(127,0,0,1), 12345)).unwrap();
        acc.soc.listen().unwrap();
        unsafe { acc.soc.async_accept(Self::on_accept, &acc); }
    }

    fn on_accept(acc: Strand<Self>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
        let (soc, ep) = res.unwrap();
        TcpServer::start(acc.io_service(), soc);
    }
}

struct TcpServer {
    soc: TcpSocket,
    buf: [u8; 1],
}

impl TcpServer {
    fn start(io: &IoService, soc: TcpSocket) {
        let mut vec = Vec::new();
        vec.push('\r' as u8);
        vec.push('\n' as u8);
        for _ in 0..10000 {
            vec.push(0x30);
        }
        vec.push('\r' as u8);
        vec.push('\n' as u8);
        soc.write_some(vec.as_slice()).unwrap();
        let sv = Strand::new(io, TcpServer {
            soc: soc,
            buf: [0],
        });

        unsafe { sv.soc.async_read_some(MutableBuffer::new(&sv.buf), Self::on_read, &sv); }
    }

    fn on_read(sv: Strand<Self>, res: io::Result<usize>) {
        assert!(res.is_err());
    }
}

struct TcpClient {
    soc: TcpSocket,
    buf: StreamBuf,
}

impl TcpClient {
    fn start(io: &IoService) {
        let cl = Strand::new(io, TcpClient {
            soc: TcpSocket::new(io, Tcp::v4()).unwrap(),
            buf: StreamBuf::new(65536),
        });
        cl.soc.connect(&TcpEndpoint::new(IpAddrV4::new(127,0,0,1), 12345)).unwrap();
        unsafe { async_read_until(&cl.soc, &cl.buf, "\r\n", Self::on_read1, &cl); }
    }

    fn on_read1(cl: Strand<Self>, res: io::Result<usize>) {
        let size = res.unwrap();
        assert_eq!(size, 2);
        unsafe { async_read_until(&cl.soc, &cl.buf, "\r\n", Self::on_read2, &cl); }
    }

    fn on_read2(mut cl: Strand<Self>, res: io::Result<usize>) {
        let size = res.unwrap();
        assert_eq!(size, 2);
        cl.buf.consume(2);
        unsafe { async_read_until(&cl.soc, &cl.buf, "\r\n", Self::on_read3, &cl); }
    }

    fn on_read3(mut cl: Strand<Self>, res: io::Result<usize>) {
        let size = res.unwrap();
        assert_eq!(size, 10002);
        unsafe { goal_flag = true; }
    }
}

#[test]
fn main() {
    let io = &IoService::new();
    TcpAcceptor::start(io);
    TcpClient::start(io);
    io.run();
    assert!(unsafe { goal_flag });
}
