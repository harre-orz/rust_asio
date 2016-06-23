# asio - ASynchronous Input/Output library for Rust

## Usage

The `asio` is not compatible to `Rust` stable version (ver 1.9). Please install a `Rust-nightly`.

This crate is on [github](https://github.com/harre-orz/rust_asio.git "github") and can be used by adding `asio` to the dependencies in your project's `Cargo.toml`.

```toml
[dependencies]
rust_asio = "0.1.1"
```

And this in your crate root:

```rust
extern crate asio;
```

For example, Connection with TCP socket code:

```rust
use asio::*;
use asio::ip::*;

struct TcpClient(TcpSocket);

impl TcpClient {
  fn start(io: &IoService) {
    let soc = Strand::new(io, TcpClient(TcpSocket::new(Tcp::v4())));
    let ep = TcpEndpoint::new((IpAddrV4::new(192.168.0.1), 12345));
    TcpSocket::async_connect(|soc|, &soc.0, &ep, Self::on_connect, &soc);
  }

  fn on_connect(soc: Strand<Self>, io::Result<()>) {
    match res {
      Ok(_) => println!("connected.");
      Err(err) => println!("{:?}", err);
    }
  }
}

fn main() {
  let io = IoService::new();
  TcpClient::start(&io);
  io.run();
}
```

## Features
 - Proactor design pattern based thread-safe asynchronous I/O.
 - Does not dependent on the number of threads.
 - Supported protocol is in TCP, UDP, Unix-domain socket and etc.
 - Supported timer is in system timer, steady timer.

## Platforms

Currently supported platforms:
 - Linux (kernel in version >=2.6.27)

## Future plans
 1. BSD will support (kqueue support).
 2. Signal will support.
 3. SSL will support.
 4. Generic protocol socket will support.
 5. File descriptor socket will support.
 6. Windows will support.
