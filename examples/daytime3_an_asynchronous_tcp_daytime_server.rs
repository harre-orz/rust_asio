extern crate asyncio;
extern crate time;

use std::io;
use asyncio::*;
use asyncio::ip::*;
use asyncio::socket_base::*;

struct DaytimeTcp {
    soc: TcpSocket,
    buf: String,
}

impl DaytimeTcp {
    fn start(ctx: &IoContext, soc: TcpSocket) {
        // Constructs a Strand wrapped TcpSocket object and buffer to transfer to the client.
        let daytime = Strand::new(
            ctx,
            DaytimeTcp {
                soc: soc,
                buf: format!("{}\r\n", time::now().ctime()),
            },
        );
        daytime.dispatch(Self::on_start);
    }

    fn on_start(daytime: Strand<Self>) {
        daytime
            .soc
            .async_write_some(daytime.buf.as_bytes(), daytime.wrap(Self::on_send));
    }

    fn on_send(_: Strand<Self>, _: io::Result<usize>) {}
}

fn on_start(sv: Strand<TcpListener>) {
    // It sets a ReuseAddr socket option.
    sv.set_option(ReuseAddr::new(true)).unwrap();

    // It binds a TCP port 13.
    sv.bind(&TcpEndpoint::new(IpAddrV4::any(), 13)).unwrap();

    // It initializes to listen.
    sv.listen().unwrap();

    // It sets asynchronous accept operation.
    sv.async_accept(sv.wrap(on_accept));
}

fn on_accept(sv: Strand<TcpListener>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
    if let Ok((soc, ep)) = res {
        println!("connected from {:?}", ep);

        // Constructs a Daytime object.
        DaytimeTcp::start(sv.as_ctx(), soc);

        // It resets asynchronous accept operation.
        sv.async_accept(sv.wrap(on_accept));
    }
}

fn main() {
    let ctx = &IoContext::new().unwrap();

    // Constructs a Strand wrapped TcpListener socket for IP version 4.
    let sv = Strand::new(ctx, TcpListener::new(ctx, Tcp::v4()).unwrap());
    sv.dispatch(on_start);

    // Runs aynchronous operations.
    ctx.run();
}
