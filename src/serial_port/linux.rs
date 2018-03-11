use ffi::{AsRawFd, RawFd};
use serial_port::{SerialPort, SerialPortOption};

use std::io;
use termios::{Termios, tcsetattr, cfsetspeed, cfgetispeed};
use termios::os::linux::*;

pub fn setup_serial(fd: RawFd) -> io::Result<Termios> {
    let mut ios = try!(Termios::from_fd(fd));
    ios.c_iflag &= !(IGNBRK | BRKINT | PARMRK | ISTRIP | INLCR | IGNCR | ICRNL | IXON);
    ios.c_oflag &= !(OPOST);
    ios.c_lflag &= !(ECHO | ECHONL | ICANON | ISIG | IEXTEN);
    ios.c_cflag &= !(CSIZE | PARENB);
    ios.c_iflag |= IGNPAR;
    ios.c_cflag |= CS8 | CREAD | CLOCAL;
    ios.c_cc[VMIN] = 0;
    ios.c_cc[VTIME] = 0;
    try!(cfsetspeed(&mut ios, B9600));
    try!(tcsetattr(fd, TCSANOW, &mut ios));
    Ok(ios)
}

#[repr(u32)]
#[derive(Clone, Copy)]
pub enum BaudRate {
    B50 = B50,
    B75 = B75,
    B110 = B110,
    B134 = B134,
    B150 = B150,
    B200 = B200,
    B300 = B300,
    B600 = B600,
    B1200 = B1200,
    B1800 = B1800,
    B2400 = B2400,
    B4800 = B4800,
    B9600 = B9600,
    B19200 = B19200,
    B38400 = B38400,
    // Extra
    B57600 = B57600,
    B115200 = B115200,
    B230400 = B230400,
    B460800 = B460800,
    B500000 = B500000,
    B576000 = B576000,
    B921600 = B921600,
    B1000000 = B1000000,
    B1152000 = B1152000,
    B1500000 = B1500000,
    B2000000 = B2000000,
    B2500000 = B2500000,
    B3000000 = B3000000,
    B3500000 = B3500000,
    B4000000 = B4000000,
}

impl SerialPortOption for BaudRate {
    fn load(target: &SerialPort) -> Self {
        let ios = &target.pimpl.data;
        match cfgetispeed(ios) {
            B50 => BaudRate::B50,
            B75 => BaudRate::B75,
            B110 => BaudRate::B110,
            B134 => BaudRate::B134,
            B150 => BaudRate::B150,
            B200 => BaudRate::B200,
            B300 => BaudRate::B300,
            B600 => BaudRate::B600,
            B1200 => BaudRate::B1200,
            B1800 => BaudRate::B1800,
            B2400 => BaudRate::B2400,
            B4800 => BaudRate::B4800,
            B9600 => BaudRate::B9600,
            B19200 => BaudRate::B19200,
            B38400 => BaudRate::B38400,
            // Extra
            B57600 => BaudRate::B57600,
            B115200 => BaudRate::B115200,
            B230400 => BaudRate::B230400,
            B460800 => BaudRate::B460800,
            B500000 => BaudRate::B500000,
            B576000 => BaudRate::B576000,
            B921600 => BaudRate::B921600,
            B1000000 => BaudRate::B1000000,
            B1152000 => BaudRate::B1152000,
            B1500000 => BaudRate::B1500000,
            B2000000 => BaudRate::B2000000,
            B2500000 => BaudRate::B2500000,
            B3000000 => BaudRate::B3000000,
            B3500000 => BaudRate::B3500000,
            B4000000 => BaudRate::B4000000,
            _ => unreachable!("invalid baud rate"),
        }
    }

    fn store(self, target: &mut SerialPort) -> io::Result<()> {
        let ios = &mut target.pimpl.data;
        cfsetspeed(ios, self as u32)
    }
}

#[repr(u32)]
#[derive(Clone, Copy)]
pub enum Parity {
    None,
    Even,
    Odd,
}

impl SerialPortOption for Parity {
    fn load(target: &SerialPort) -> Self {
        let ios = &target.pimpl.data;
        if (ios.c_cflag & PARENB) == 0 {
            Parity::None
        } else if (ios.c_cflag & PARODD) == 0 {
            Parity::Even
        } else {
            Parity::Odd
        }
    }

    fn store(self, target: &mut SerialPort) -> io::Result<()> {
        let fd = target.as_raw_fd();
        let ios = &mut target.pimpl.data;
        match self {
            Parity::None => {
                ios.c_iflag |= IGNPAR;
                ios.c_cflag &= !(PARENB | PARODD);
            }
            Parity::Even => {
                ios.c_iflag &= !(IGNPAR | PARMRK);
                ios.c_iflag |= INPCK;
                ios.c_cflag |= PARENB;
                ios.c_cflag &= !PARODD;
            }
            Parity::Odd => {
                ios.c_iflag &= !(IGNPAR | PARMRK);
                ios.c_iflag |= INPCK;
                ios.c_cflag |= PARENB | PARODD;
            }
        }
        tcsetattr(fd, TCSANOW, ios)
    }
}

#[repr(u32)]
#[derive(Clone, Copy)]
pub enum CSize {
    CS5 = CS5,
    CS6 = CS6,
    CS7 = CS7,
    CS8 = CS8,
}

impl SerialPortOption for CSize {
    fn load(target: &SerialPort) -> Self {
        let ios = &target.pimpl.data;
        match ios.c_cflag & CSIZE {
            CS5 => CSize::CS5,
            CS6 => CSize::CS6,
            CS7 => CSize::CS7,
            CS8 => CSize::CS8,
            _ => unreachable!("invalid charactor size"),
        }
    }

    fn store(self, target: &mut SerialPort) -> io::Result<()> {
        let fd = target.as_raw_fd();
        let ios = &mut target.pimpl.data;
        ios.c_cflag &= !CSIZE;
        ios.c_cflag |= self as u32;
        tcsetattr(fd, TCSANOW, ios)
    }
}

#[repr(u32)]
#[derive(Clone, Copy)]
pub enum FlowControl {
    None,
    Software,
    Hardware,
}

impl SerialPortOption for FlowControl {
    fn load(target: &SerialPort) -> Self {
        let ios = &target.pimpl.data;
        if (ios.c_iflag & (IXOFF | IXON)) != 0 {
            FlowControl::Software
        } else if (ios.c_cflag & CRTSCTS) != 0 {
            FlowControl::Hardware
        } else {
            FlowControl::None
        }
    }

    fn store(self, target: &mut SerialPort) -> io::Result<()> {
        let fd = target.as_raw_fd();
        let ios = &mut target.pimpl.data;
        match self {
            FlowControl::None => {
                ios.c_iflag &= !(IXOFF | IXON);
                ios.c_cflag &= !CRTSCTS;
            }
            FlowControl::Software => {
                ios.c_iflag |= IXOFF | IXON;
                ios.c_cflag &= !CRTSCTS;
            }
            FlowControl::Hardware => {
                ios.c_iflag &= !(IXOFF | IXON);
                ios.c_cflag |= CRTSCTS;
            }
        }
        tcsetattr(fd, TCSANOW, ios)
    }
}

#[repr(u32)]
#[derive(Clone, Copy)]
pub enum StopBits {
    One,
    Two,
}

impl SerialPortOption for StopBits {
    fn load(target: &SerialPort) -> Self {
        let ios = &target.pimpl.data;
        if (ios.c_cflag & CSTOPB) == 0 {
            StopBits::One
        } else {
            StopBits::Two
        }
    }

    fn store(self, target: &mut SerialPort) -> io::Result<()> {
        let fd = target.as_raw_fd();
        let ios = &mut target.pimpl.data;
        match self {
            StopBits::One => ios.c_cflag &= !CSTOPB,
            StopBits::Two => ios.c_cflag |= CSTOPB,
        }
        tcsetattr(fd, TCSANOW, ios)
    }
}
