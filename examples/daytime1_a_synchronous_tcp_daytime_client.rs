extern crate asyncio;

use std::env::args;
use std::process::exit;
use std::str;
use asyncio::*;
use asyncio::ip::*;

fn main() {
    let host = args().nth(1).unwrap_or_else(|| {
        println!("usage: client <host>");
        exit(1);
    });

    // IoContext に関連するすべてのオブジェクトの基を最初に作成します。
    let ctx = &IoContext::new().unwrap();

    // TCPの名前解決をするオブジェクトを作成します。
    let res = TcpResolver::new(ctx);

    // 名前解決を解決し、接続したソケットと接続先エンドポイントを返します。
    let (soc, ep) = res.connect((&host[..], "daytime")).unwrap();
    println!("connected to {}", ep);

    // TCPソケットの接続先からメッセージを読み込みます。
    let mut buf = [0; 256];
    let len = soc.read_some(&mut buf).unwrap();

    println!("{}", str::from_utf8(&buf[..len]).unwrap());
}
