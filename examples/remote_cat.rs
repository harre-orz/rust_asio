extern crate asyncio;
use std::env;
use std::sync::{Arc, Mutex};
use std::io::{self, Read};
use std::fs::File;
use std::str::from_utf8;
use std::thread;
use std::time::Duration;

use asyncio::*;
use asyncio::ip::*;
use asyncio::socket_base::*;

static mut USE_LINUM: bool = false;

fn on_accept(sv: Arc<Mutex<TcpListener>>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
    if let Ok((soc, ep)) = res {
        spawn(sv.lock().unwrap().as_ctx(), move |coro| {
            println!("connected from {}", ep);
            loop {
                let mut buf = [0; 256];
                let len = soc.async_read_some(&mut buf, coro.wrap()).unwrap();
                let filename = String::from(from_utf8(&buf[..len - 2]).unwrap()).replace("/", "_");
                println!("receive filename={}, len={}", filename, len);
                if let Ok(mut fs) = File::open(filename) {
                    let mut str = String::new();
                    let mut num = 0;
                    if let Ok(_) = fs.read_to_string(&mut str) {
                        let lines: Vec<&str> = str.split('\n').collect();
                        for line in lines {
                            if unsafe { USE_LINUM } {
                                num += 1;
                                let num_buf = format!("{} ", num);
                                soc.async_write_some(num_buf.as_bytes(), coro.wrap())
                                    .unwrap();
                            }
                            if line.len() > 0 {
                                soc.async_write_some(line.as_bytes(), coro.wrap()).unwrap();
                            }
                            soc.async_write_some("\r\n".as_bytes(), coro.wrap())
                                .unwrap();
                            thread::sleep(Duration::new(0, 100000000));
                        }
                    }
                } else {
                    let msg = "Not found filename\r\n";
                    soc.async_write_some(msg.as_bytes(), coro.wrap()).unwrap();
                }
            }
        });
    }
    start_server(sv)
}

fn start_server(sv: Arc<Mutex<TcpListener>>) {
    sv.lock().unwrap().async_accept(wrap(on_accept, &sv))
}

fn main() {
    let ctx = &IoContext::new().unwrap();

    for arg in env::args().skip(1).into_iter() {
        if arg == "-n" {
            unsafe {
                USE_LINUM = true;
            }
        }
    }

    let sv = TcpListener::new(ctx, Tcp::v4()).unwrap();
    sv.set_option(ReuseAddr::new(true)).unwrap();
    sv.bind(&TcpEndpoint::new(Tcp::v4(), 12345)).unwrap();
    sv.listen().unwrap();
    start_server(Arc::new(Mutex::new(sv)));

    ctx.run();
}
