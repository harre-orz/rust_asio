use asyncio::IoContext;
use asyncio::ip::{TcpEndpoint, TcpListener};
use asyncio::socket_base::ReuseAddr;
use std::net::Ipv4Addr;

fn ctime() -> String {
    use chrono::Local;

    Local::now().format("%a %b %_d %H:%M:%S %Y").to_string()
}

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

fn server(ctx: IoContext) -> Result {
    // Constructs a TcpListener socket for IP version 4 and binds a TCP port 13.
    let soc = TcpListener::new(&ctx)
        // It sets a ReuseAddr socket option.
        .set_option(ReuseAddr::ON)
        // It initializes to listen.
        .listen(&TcpEndpoint::v4(Ipv4Addr::UNSPECIFIED, 13))?;
    // It waits for accepted by a client connection.
    while let Ok((acc, ep)) = soc.accept() {
        println!("connected from {:?}", ep);

        // A client is accessing our program.
        // Makes the current time and transfer to the client.
        let buf = format!("{}\r\n", ctime());
        acc.write_some(buf.as_bytes())?;
    }
    Ok(())
}

fn main() -> Result {
    let ctx = IoContext::new()?;
    server(ctx)
}
