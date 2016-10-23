use std::io;
use std::ffi::CString;
use libc::{self, O_RDWR, O_NOCTTY, O_NDELAY, O_NONBLOCK, O_CLOEXEC};
use termios::{Termios, tcsetattr, tcsendbreak, cfgetispeed, cfsetspeed};
use termios::os::linux::*;
use error::{invalid_argument};
use io_service::{IoObject, IoService, RawFd, AsRawFd, IoActor, Handler};
use stream::Stream;
use fd_ops::{AsIoActor, cancel, read, write, async_read, async_write};

pub trait SerialPortOption : Sized {
    fn load(target: &SerialPort) -> Self;

    fn store(self, target: &mut SerialPort) -> io::Result<()>;
}

#[derive(Clone, Copy)]
pub enum BaudRate {
    B50 = B50 as isize,
    B75 = B75 as isize,
    B110 = B110 as isize,
    B134 = B134 as isize,
    B150 = B150 as isize,
    B200 = B200 as isize,
    B300 = B300 as isize,
    B600 = B600 as isize,
    B1200 = B1200 as isize,
    B1800 = B1800 as isize,
    B2400 = B2400 as isize,
    B4800 = B4800 as isize,
    B9600 = B9600 as isize,
    B19200 = B19200 as isize,
    B38400 = B38400 as isize,
    // Extra
    B57600 = B57600 as isize,
    B115200 = B115200 as isize,
    B230400 = B230400 as isize,
    B460800 = B460800 as isize,
    B500000 = B500000 as isize,
    B576000 = B576000 as isize,
    B921600 = B921600 as isize,
    B1000000 = B1000000 as isize,
    B1152000 = B1152000 as isize,
    B1500000 = B1500000 as isize,
    B2000000 = B2000000 as isize,
    B2500000 = B2500000 as isize,
    B3000000 = B3000000 as isize,
    B3500000 = B3500000 as isize,
    B4000000 = B4000000 as isize,
}

impl SerialPortOption for BaudRate {
    fn load(target: &SerialPort) -> Self {
        match cfgetispeed(&target.ios) {
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
            _ => unreachable!("invalid baud rate"),
        }
    }

    fn store(self, target: &mut SerialPort) -> io::Result<()> {
        try!(cfsetspeed(&mut target.ios, self as u32));
        Ok(())
    }
}

pub enum FlowControl {
    None,
    Software,
    Hardware,
}

impl SerialPortOption for FlowControl {
    fn load(target: &SerialPort) -> Self {
        use termios::os::linux::CRTSCTS;

        if (target.ios.c_iflag & (IXOFF | IXON)) != 0{
            FlowControl::Software
        } else if (target.ios.c_cflag & CRTSCTS) != 0 {
            FlowControl::Hardware
        } else {
            FlowControl::None
        }
    }

    fn store(self, target: &mut SerialPort) -> io::Result<()> {
        use termios::os::linux::CRTSCTS;

        match self {
            FlowControl::None => {
                target.ios.c_iflag &= !(IXOFF | IXON);
                target.ios.c_cflag &= !CRTSCTS;
            },
            FlowControl::Software => {
                target.ios.c_iflag |= IXOFF | IXON;
                target.ios.c_cflag &= !CRTSCTS;
            },
            FlowControl::Hardware => {
                target.ios.c_iflag &= !(IXOFF | IXON);
                target.ios.c_cflag |= CRTSCTS;
            },
        }

        tcsetattr(target.as_raw_fd(), TCSANOW, &mut target.ios)
    }
}

pub enum Parity {
    None,
    Even,
    Odd,
}

impl SerialPortOption for Parity {
    fn load(target: &SerialPort) -> Self {
        if (target.ios.c_cflag & PARENB) == 0 {
            Parity::None
        } else if (target.ios.c_cflag & PARODD) == 0 {
            Parity::Even
        } else {
            Parity::Odd
        }
    }

    fn store(self, target: &mut SerialPort) -> io::Result<()> {
        match self {
            Parity::None => {
                target.ios.c_iflag |= IGNPAR;
                target.ios.c_cflag &= !(PARENB | PARODD);
            },
            Parity::Even => {
                target.ios.c_iflag &= !(IGNPAR | PARMRK);
                target.ios.c_iflag |= INPCK;
                target.ios.c_cflag |= PARENB;
                target.ios.c_cflag &= !PARODD;
            },
            Parity::Odd => {
                target.ios.c_iflag &= !(IGNPAR | PARMRK);
                target.ios.c_iflag |= INPCK;
                target.ios.c_cflag |= PARENB | PARODD;
            },
        }

        tcsetattr(target.as_raw_fd(), TCSANOW, &mut target.ios)
    }

}

pub enum StopBits {
    One,
    Two,
}

impl SerialPortOption for StopBits {
    fn load(target: &SerialPort) -> Self {
        if (target.ios.c_cflag & CSTOPB) == 0 {
            StopBits::One
        } else {
            StopBits::Two
        }
    }

    fn store(self, target: &mut SerialPort) -> io::Result<()> {
        match self {
            StopBits::One => target.ios.c_cflag &= !CSTOPB,
            StopBits::Two => target.ios.c_cflag |= CSTOPB,
        }

        tcsetattr(target.as_raw_fd(), TCSANOW, &mut target.ios)
    }
}

pub enum CSize {
    CS5 = CS5 as isize,
    CS6 = CS6 as isize,
    CS7 = CS7 as isize,
    CS8 = CS8 as isize,
}

impl SerialPortOption for CSize {
    fn load(target: &SerialPort) -> Self {
        match target.ios.c_cflag & CSIZE {
            CS5 => CSize::CS5,
            CS6 => CSize::CS6,
            CS7 => CSize::CS7,
            CS8 => CSize::CS8,
            _ => unreachable!("invalid charactor size"),
        }
    }

    fn store(self, target: &mut SerialPort) -> io::Result<()> {
        target.ios.c_cflag &= !CSIZE;
        target.ios.c_cflag |= self as u32;
        tcsetattr(target.as_raw_fd(), TCSANOW, &mut target.ios)
    }
}

pub struct SerialPort {
    act: IoActor,
    ios: Termios,
}

impl SerialPort {
    fn setup(fd: RawFd) -> io::Result<Termios> {
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

    pub fn new<T>(io: &IoService, device: T) -> io::Result<SerialPort>
        where T: AsRef<str>
    {
        let fd = match CString::new(device.as_ref()) {
            Ok(device) => libc_try!(libc::open(
                device.as_bytes_with_nul().as_ptr() as *const i8,
                O_RDWR | O_NOCTTY | O_NDELAY | O_NONBLOCK | O_CLOEXEC)
            ),
            _ => return Err(invalid_argument()),
        };
        Ok(SerialPort {
            act: IoActor::new(io, fd),
            ios: try!(Self::setup(fd)),
        })
    }

    pub fn cancel(&self) {
        cancel(self)
    }

    pub fn get_option<C>(&self) -> C
        where C: SerialPortOption,
    {
        C::load(self)
    }

    pub fn send_break(&self) -> io::Result<()> {
        tcsendbreak(self.as_raw_fd(), 0)
    }

    pub fn set_option<C>(&mut self, cmd: C) -> io::Result<()>
        where C: SerialPortOption,
    {
        cmd.store(self)
    }
}

unsafe impl IoObject for SerialPort {
    fn io_service(&self) -> &IoService {
        self.act.io_service()
    }
}

impl Stream for SerialPort {
    fn async_read_some<F>(&self, buf: &mut [u8], handler: F) -> F::Output
        where F: Handler<usize>
    {
        async_read(self, buf, handler)
    }

    fn async_write_some<F>(&self, buf: &[u8], handler: F) -> F::Output
        where F: Handler<usize>
    {
        async_write(self, buf, handler)
    }

    fn read_some(&self, buf: &mut [u8]) -> io::Result<usize> {
        read(self, buf)
    }

    fn write_some(&self, buf: &[u8]) -> io::Result<usize> {
        write(self, buf)
    }
}

impl AsRawFd for SerialPort {
    fn as_raw_fd(&self) -> RawFd {
        self.act.as_raw_fd()
    }
}

impl AsIoActor for SerialPort {
    fn as_io_actor(&self) -> &IoActor {
        &self.act
    }
}

#[test]
fn test_baud_rate() {
    assert_eq!(BaudRate::B50 as u32, B50);
    assert_eq!(BaudRate::B9600 as u32, B9600);
    assert_eq!(BaudRate::B38400 as u32, B38400);
}

#[test]
#[ignore]
fn test_serial_port() {
    let io = &IoService::new();
    let mut serial_port = SerialPort::new(io, "/dev/ttyS0").unwrap();
    serial_port.set_option(BaudRate::B9600).unwrap();
}
