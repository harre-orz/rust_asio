extern crate futures;

use crate::futures::executor::{block_on, ThreadPool};
use asyio::ip::TcpResolver;
use asyio::IoContext;
use std::env::args;
use std::process::exit;
use std::str;

fn main() {
    let host: String = args().nth(1).unwrap_or_else(|| {
        println!("usage: client <host>");
        exit(1);
    });
    let ctx = IoContext::new().unwrap();
    let pool = ThreadPool::new().unwrap();

    block_on(async {
        let ctx_clone = ctx.clone();
        pool.spawn_ok(async move {
            // Constructs a TcpResolver for IP version 4.
            let res = TcpResolver::v4(&ctx);

            // It connects resolved endpoints.
            let (soc, ep) = res.async_connect((host, "daytime")).await.unwrap();
            println!("connected to {:?}", ep);

            // A server is send message to out program.
            let mut buf = [0; 256];
            let len = soc.async_read_some(&mut buf).await.unwrap();
            println!("{}", str::from_utf8(&buf[..len]).unwrap());
        });
        let _ = ctx_clone.run().await;
    });
}
