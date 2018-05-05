extern crate asyncio;

use std::io::{self, Write};
use std::env::args;
use std::process::exit;
use std::sync::Arc;
use std::str::from_utf8;
use asyncio::*;
use asyncio::ip::*;
use asyncio::socket_base::*;

struct HttpSession {
    soc: TcpSocket,
    buf: StreamBuf,
}

impl HttpSession {
    fn start(ctx: &IoContext, soc: TcpSocket) {
        let http = Strand::new(
            ctx,
            HttpSession {
                soc: soc,
                buf: StreamBuf::new(),
            },
        );
        http.dispatch(Self::on_start);
    }

    fn on_start(http: Strand<Self>) {
        http.soc.async_read_until(
            &mut http.get().buf,
            "\r\n",
            http.wrap(Self::on_request_line),
        );
    }

    fn on_request_line(mut http: Strand<Self>, res: io::Result<usize>) {
        if let Ok(size) = res {
            println!(
                "({}) request line: {:?}",
                size,
                from_utf8(&http.buf.as_bytes()[..size - 2]).unwrap()
            );

            http.buf.consume(size);
            http.soc.async_read_until(
                &mut http.get().buf,
                "\r\n",
                http.wrap(Self::on_request_header),
            );
        }
    }

    fn on_request_header(mut http: Strand<Self>, res: io::Result<usize>) {
        if let Ok(size) = res {
            if size > 2 {
                println!(
                    "({}) request header: {:?}",
                    size,
                    from_utf8(&http.buf.as_bytes()[..size - 2]).unwrap()
                );

                http.buf.consume(size);
                http.soc.async_read_until(
                    &mut http.get().buf,
                    "\r\n",
                    http.wrap(Self::on_request_header),
                );
            } else {
                let len = http.buf.len();
                http.buf.consume(len);

                let len = http.buf
                    .write(
                        "HTTP/1.1 200 OK\r\n\
Connection: close\r\n\
Content-type: text/html\r\n\
Content-Length: 4\r\n\
\r\n\
hoge"
                            .as_bytes(),
                    )
                    .unwrap();
                http.soc.async_write_until(
                    &mut http.get().buf,
                    len,
                    http.wrap(Self::on_response),
                );
            }
        }
    }

    fn on_response(mut _http: Strand<Self>, res: io::Result<usize>) {
        if let Ok(size) = res {
            println!("({}) response", size);
        }
    }
}

fn on_accept(soc: Arc<TcpListener>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
    if soc.as_ctx().stopped() {
        return;
    }

    if let Ok((acc, ep)) = res {
        println!("connected from {}", ep);
        HttpSession::start(soc.as_ctx(), acc);
    }

    soc.async_accept(wrap(&soc, on_accept));
}

fn main() {
    let port = args().nth(1).unwrap_or_else(|| {
        println!("usage: http_server <port>");
        exit(1);
    });
    let port = u16::from_str_radix(&port, 10).unwrap();
    let ep = TcpEndpoint::new(IpAddrV6::any(), port);

    let ctx = &IoContext::new().unwrap();
    let soc = Arc::new(TcpListener::new(ctx, ep.protocol()).unwrap());
    soc.set_option(ReuseAddr::new(true)).unwrap();
    soc.bind(&ep).unwrap();
    soc.listen().unwrap();

    println!("start {}", ep);
    soc.async_accept(wrap(&soc, on_accept));

    ctx.run();
}
