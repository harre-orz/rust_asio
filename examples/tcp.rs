extern crate asyio;

use asyio::ip::*;
use asyio::IoContext;
use std::net::Ipv4Addr;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = &IoContext::new();

    //let ep = TcpEndpoint::from((Ipv4Addr::LOCALHOST, 12345));
    //let tcp = TcpListener::new(ctx, Tcp::V4).listen()?;

    //tcp.accept()?;

    Ok(())
}
