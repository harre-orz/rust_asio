use std::io;

pub trait SerialPortOption : Sized {
    fn load(target: &SerialPort) -> Self;

    fn store(self, target: &mut SerialPort) -> io::Result<()>;
}

#[cfg(feature = "termios")] mod termios;
#[cfg(feature = "termios")] pub use self::termios::SerialPort;

#[cfg(target_os = "linux")] mod linux;
#[cfg(target_os = "linux")] pub use self::linux::{BaudRate, Parity, CSize, FlowControl, StopBits};

#[cfg(target_os = "macos")] mod macos;
#[cfg(target_os = "macos")] pub use self::macos::{BaudRate, Parity, CSize, FlowControl, StopBits};
