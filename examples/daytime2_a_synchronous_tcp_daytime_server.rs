extern crate asyio;
extern crate chrono;

use asyio::{YieldContext, IoContext};
use asyio::ip::{Tcp, TcpEndpoint, TcpListener};
use std::net::Ipv4Addr;

fn ctime() -> String {
    use chrono::Local;

    Local::now().format("%a %b %_d %H:%M:%S %Y").to_string()
}

fn daytime_server(ctx: &mut YieldContext) -> Result<(), Box<dyn std::error::Error>> {
    // Constructs a TcpListener socket for IP version 4.
    let soc = TcpListener::new(ctx.as_ctx(), Tcp::V4)
        // It sets a ReuseAddr socket option.
        .reuse_addr(true)

        // It binds a TCP port 13.
        .bind(TcpEndpoint::v4(Ipv4Addr::UNSPECIFIED, 13))

        // It initializes to listen.
        .listen()?;

    // It waits for accepted by a client connection.
    while let Ok((acc, ep)) = soc.accept(ctx) {
        println!("connected from {:?}", ep);

        // A client is accessing our program.
        // Makes the current time and transfer to the client.
        let buf = format!("{}\r\n", ctime());
        acc.write_some(buf.as_bytes(), ctx)?;
    }

    Ok(())
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = &IoContext::new()?;
    ctx.spawn(|ctx| {
        let _ = daytime_server(ctx);
    })?;
    ctx.run();

    Ok(())
}
