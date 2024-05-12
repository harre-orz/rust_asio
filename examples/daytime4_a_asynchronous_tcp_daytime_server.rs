use asyio::ip::{Tcp, TcpEndpoint, TcpListener};
use asyio::IoContext;
use futures::executor::LocalPool;
use futures::task::SpawnExt;
use std::net::Ipv4Addr;

fn ctime() -> String {
    use chrono::Local;

    Local::now().format("%a %b %_d %H:%M:%S %Y").to_string()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = IoContext::new()?;
    let mut pool = LocalPool::new();
    {
        let ctx = ctx.clone();
        pool.spawner().spawn(async move {
            async {
                // Constructs a TcpListener socket for IP version 4.
                let soc = TcpListener::new(&ctx, Tcp::V4)?
                    // It sets a ReuseAddr socket option.
                    .reuse_addr(true)?
                    // It binds a TCP port 13.
                    .bind(&TcpEndpoint::v4(Ipv4Addr::UNSPECIFIED, 13))?
                    // It initializes to listen.
                    .listen_async()?;
                while let Ok((soc, ep)) = soc.async_accept().await {
                    println!("connected from {:?}", ep);

                    // A client is accessing our program.
                    // Makes the current time and transfer to the client.
                    let buf = format!("{}\r\n", ctime());
                    soc.async_write_some(buf.as_bytes()).await?;
                }
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await
            .unwrap()
        })?
    }
    {
        let ctx = ctx.clone();
        pool.spawner().spawn(async move {
            ctx.run().await.unwrap();
        })?
    }
    pool.run();
    Ok(())
}
