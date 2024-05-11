extern crate futures;

use crate::futures::executor::{block_on, ThreadPool};
use asyio::ip::{Tcp, TcpEndpoint, TcpListener};
use asyio::stream::AsyncStreamSocket;
use asyio::IoContext;
use std::env::args;
use std::net::Ipv4Addr;
use std::process::exit;
use std::str;

fn ctime() -> String {
    use chrono::Local;

    Local::now().format("%a %b %_d %H:%M:%S %Y").to_string()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = ThreadPool::new()?;
    let ctx = &IoContext::new()?;

    // // Constructs a TcpListener socket for IP version 4.
    // let soc = TcpListener::new(&ctx, Tcp::V4)?
    //     // It sets a ReuseAddr socket option.
    //     .reuse_addr(true)?
    //     // It binds a TCP port 13.
    //     .bind(&TcpEndpoint::v4(Ipv4Addr::UNSPECIFIED, 13))?
    //     // It initializes to listen.
    //     .listen::<AsyncStreamSocket<Tcp>>()?
    //     .into();
    // block_on(async move {
    //     while let Ok((soc, ep)) = soc.async_accept().await {
    //         println!("connected from {:?}", ep);
    //
    //         // A client is accessing our program.
    //         // Makes the current time and transfer to the client.
    //         let buf = format!("{}\r\n", ctime());
    //         soc.async_write_some(buf.as_bytes()).await.unwrap();
    //     }
    // });
    Ok(())
}
