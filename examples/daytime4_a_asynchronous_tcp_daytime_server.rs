use asyio::IoContext;
use asyio::ip::{Tcp, TcpEndpoint, TcpListener};
use futures::executor::LocalPool;
use futures::task::SpawnExt;
use std::net::Ipv4Addr;

fn ctime() -> String {
    use chrono::Local;

    Local::now().format("%a %b %_d %H:%M:%S %Y").to_string()
}

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

async fn server(ctx: IoContext) -> Result {
    // Constructs a TcpListener socket for IP version 4.
    let soc = TcpListener::new(&ctx, Tcp::V4)?
        // It sets a ReuseAddr socket option.
        .reuse_addr(true)?
        // It binds a TCP port 13.
        .bind(&TcpEndpoint::v4(Ipv4Addr::UNSPECIFIED, 13))?
        // It initializes to listen.
        .listen_async()?;
    // It waits for accepted by a client connection.
    while let Ok((soc, ep)) = soc.async_accept().await {
        println!("connected from {:?}", ep);

        // A client is accessing our program.
        // Makes the current time and transfer to the client.
        let buf = format!("{}\r\n", ctime());
        soc.async_write_some(buf.as_bytes()).await?;
    }
    Ok(())
}

fn main() -> Result {
    let ctx = IoContext::new()?;
    let mut pool = LocalPool::new();
    {
        let ctx = ctx.clone();
        pool.spawner()
            .spawn(async move { server(ctx).await.unwrap() })?;
    }
    pool.spawner().spawn(async move {
        ctx.run().await.unwrap();
    })?;
    pool.run();
    Ok(())
}
