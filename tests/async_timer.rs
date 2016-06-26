extern crate time;
extern crate asio;
use std::io;
use asio::*;
use time::Duration;

struct FooTimer {
    timer: SteadyTimer,
}

impl FooTimer {
    fn start(io: &IoService) {
        let obj = Strand::new(io, FooTimer {
            timer: SteadyTimer::new(io),
        });
        SteadyTimer::async_wait_for(|obj| &obj.timer, &Duration::nanoseconds(1), Self::on_nano_wait, &obj);
    }

    fn on_nano_wait(obj: Strand<Self>, res: io::Result<()>) {
        if let Ok(_) = res {
            SteadyTimer::async_wait_for(|obj| &obj.timer, &Duration::milliseconds(2), Self::on_milli_wait, &obj);
        } else {
            panic!();
        }
    }

    fn on_milli_wait(obj: Strand<Self>, res: io::Result<()>) {
        if let Ok(_) = res {
            SteadyTimer::async_wait_for(|obj| &obj.timer, &Duration::seconds(3), Self::on_sec_wait, &obj);
        } else {
            panic!();
        }
    }

    fn on_sec_wait(_: Strand<Self>, res: io::Result<()>) {
        if let Ok(_) = res {
        } else {
            panic!();
        }
    }
}

#[test]
fn main() {
    let io = IoService::new();
    FooTimer::start(&io);
    io.run();
}
