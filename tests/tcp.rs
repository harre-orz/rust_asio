use asyio::ip::Tcp;
use asyio::IoContext;
use std::net::Ipv4Addr;

// fn main() -> Result<(), Box<dyn std::error::Error>> {
//     let ctx = &IoContext::new()?;
//     let ep = TcpEndpoint::v4(Ipv4Addr::UNSPECIFIED, 12345);
//     let tcp = TcpListener::new(ctx, Tcp::V4)
//         .reuse_addr(true)
//         .bind(ep)
//         .listen()?;
//     Ok(tcp.close()?)
// }
