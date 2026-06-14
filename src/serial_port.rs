use crate::core::IoContext;
use crate::error::{OsError, Result};
use crate::primitive::{Fd, Socket, Timeout};
use crate::socket::AsyncSocket;
use std::ffi::CStr;
use std::mem::MaybeUninit;
use std::time::Duration;

pub trait SerialPortOpt: Sized {
    fn load(ios: &libc::termios) -> Self;

    fn store(self, ios: &mut libc::termios, soc: &Socket) -> Result<()>;
}

#[repr(u32)]
#[derive(Clone, Copy)]
#[cfg(target_os = "linux")]
pub enum BaudRate {
    B50 = libc::B50,
    B75 = libc::B75,
    B110 = libc::B110,
    B134 = libc::B134,
    B150 = libc::B150,
    B200 = libc::B200,
    B300 = libc::B300,
    B600 = libc::B600,
    B1200 = libc::B1200,
    B1800 = libc::B1800,
    B2400 = libc::B2400,
    B4800 = libc::B4800,
    B9600 = libc::B9600,
    B19200 = libc::B19200,
    B38400 = libc::B38400,
    // Extra
    B57600 = libc::B57600,
    B115200 = libc::B115200,
    B230400 = libc::B230400,
    B460800 = libc::B460800,
    B500000 = libc::B500000,
    B576000 = libc::B576000,
    B921600 = libc::B921600,
    B1000000 = libc::B1000000,
    B1152000 = libc::B1152000,
    B1500000 = libc::B1500000,
    B2000000 = libc::B2000000,
    B2500000 = libc::B2500000,
    B3000000 = libc::B3000000,
    B3500000 = libc::B3500000,
    B4000000 = libc::B4000000,
}

#[repr(u64)]
#[derive(Clone, Copy)]
#[cfg(target_os = "macos")]
pub enum BaudRate {
    B50 = libc::B50,
    B75 = libc::B75,
    B110 = libc::B110,
    B134 = libc::B134,
    B150 = libc::B150,
    B200 = libc::B200,
    B300 = libc::B300,
    B600 = libc::B600,
    B1200 = libc::B1200,
    B1800 = libc::B1800,
    B2400 = libc::B2400,
    B4800 = libc::B4800,
    B9600 = libc::B9600,
    B19200 = libc::B19200,
    B38400 = libc::B38400,
    // Extra
    B57600 = libc::B57600,
    B115200 = libc::B115200,
    B230400 = libc::B230400,
}

#[repr(u32)]
#[derive(Clone, Copy)]
pub enum Parity {
    None,
    Even,
    Odd,
}

#[repr(u32)]
#[derive(Clone, Copy)]
#[cfg(target_os = "linux")]
pub enum CSize {
    CS5 = libc::CS5,
    CS6 = libc::CS6,
    CS7 = libc::CS7,
    CS8 = libc::CS8,
}

#[repr(u64)]
#[derive(Clone, Copy)]
#[cfg(target_os = "macos")]
pub enum CSize {
    CS5 = libc::CS5,
    CS6 = libc::CS6,
    CS7 = libc::CS7,
    CS8 = libc::CS8,
}

#[derive(Clone, Copy)]
pub enum FlowControl {
    None,
    Software,
    Hardware,
}

#[derive(Clone, Copy)]
pub enum StopBits {
    One,
    Two,
}

fn setup_termios(fd: &Fd) -> Result<libc::termios> {
    let mut ios = MaybeUninit::<libc::termios>::uninit();
    unsafe {
        match libc::tcgetattr(fd.as_raw_fd(), ios.as_mut_ptr()) {
            -1 => Err(OsError::last()),
            _ => Ok(ios.assume_init()),
        }
    }
}

fn tcsendbreak(fd: &Fd, duration: i32) -> Result<()> {
    unsafe {
        match libc::tcsendbreak(fd.as_raw_fd(), duration) {
            -1 => Err(OsError::last()),
            _ => Ok(()),
        }
    }
}

impl SerialPortOpt for BaudRate {
    fn load(ios: &libc::termios) -> Self {
        match unsafe { libc::cfgetispeed(ios) } {
            libc::B50 => BaudRate::B50,
            libc::B75 => BaudRate::B75,
            libc::B110 => BaudRate::B110,
            libc::B134 => BaudRate::B134,
            libc::B150 => BaudRate::B150,
            libc::B200 => BaudRate::B200,
            libc::B300 => BaudRate::B300,
            libc::B600 => BaudRate::B600,
            libc::B1200 => BaudRate::B1200,
            libc::B1800 => BaudRate::B1800,
            libc::B2400 => BaudRate::B2400,
            libc::B4800 => BaudRate::B4800,
            libc::B9600 => BaudRate::B9600,
            libc::B19200 => BaudRate::B19200,
            libc::B38400 => BaudRate::B38400,
            // Extra
            libc::B57600 => BaudRate::B57600,
            libc::B115200 => BaudRate::B115200,
            libc::B230400 => BaudRate::B230400,
            #[cfg(target_os = "linux")]
            libc::B460800 => BaudRate::B460800,
            #[cfg(target_os = "linux")]
            libc::B500000 => BaudRate::B500000,
            #[cfg(target_os = "linux")]
            libc::B576000 => BaudRate::B576000,
            #[cfg(target_os = "linux")]
            libc::B921600 => BaudRate::B921600,
            #[cfg(target_os = "linux")]
            libc::B1000000 => BaudRate::B1000000,
            #[cfg(target_os = "linux")]
            libc::B1152000 => BaudRate::B1152000,
            #[cfg(target_os = "linux")]
            libc::B1500000 => BaudRate::B1500000,
            #[cfg(target_os = "linux")]
            libc::B2000000 => BaudRate::B2000000,
            #[cfg(target_os = "linux")]
            libc::B2500000 => BaudRate::B2500000,
            #[cfg(target_os = "linux")]
            libc::B3000000 => BaudRate::B3000000,
            #[cfg(target_os = "linux")]
            libc::B3500000 => BaudRate::B3500000,
            #[cfg(target_os = "linux")]
            libc::B4000000 => BaudRate::B4000000,
            _ => unreachable!("invalid baud rate"),
        }
    }

    fn store(self, ios: &mut libc::termios, _: &Socket) -> Result<()> {
        unsafe {
            match libc::cfsetspeed(ios, self as libc::speed_t) {
                -1 => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }
}

impl SerialPortOpt for CSize {
    fn load(ios: &libc::termios) -> Self {
        match ios.c_cflag & libc::CSIZE {
            libc::CS5 => CSize::CS5,
            libc::CS6 => CSize::CS6,
            libc::CS7 => CSize::CS7,
            libc::CS8 => CSize::CS8,
            _ => unreachable!("invalid character size"),
        }
    }

    fn store(self, ios: &mut libc::termios, soc: &Socket) -> Result<()> {
        ios.c_cflag &= !libc::CSIZE;
        ios.c_cflag |= self as libc::tcflag_t;
        unsafe {
            match libc::tcsetattr(soc.0.as_raw_fd(), libc::TCSANOW, ios) {
                -1 => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }
}

impl SerialPortOpt for FlowControl {
    fn load(ios: &libc::termios) -> Self {
        if (ios.c_iflag & (libc::IXOFF | libc::IXON)) != 0 {
            FlowControl::Software
        } else if (ios.c_cflag & libc::CRTSCTS) != 0 {
            FlowControl::Hardware
        } else {
            FlowControl::None
        }
    }

    fn store(self, ios: &mut libc::termios, soc: &Socket) -> Result<()> {
        match self {
            FlowControl::None => {
                ios.c_iflag &= !(libc::IXOFF | libc::IXON);
                ios.c_cflag &= !libc::CRTSCTS;
            }
            FlowControl::Software => {
                ios.c_iflag |= libc::IXOFF | libc::IXON;
                ios.c_cflag &= !libc::CRTSCTS;
            }
            FlowControl::Hardware => {
                ios.c_iflag &= !(libc::IXOFF | libc::IXON);
                ios.c_cflag |= libc::CRTSCTS;
            }
        }
        unsafe {
            match libc::tcsetattr(soc.0.as_raw_fd(), libc::TCSANOW, ios) {
                -1 => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }
}

impl SerialPortOpt for Parity {
    fn load(ios: &libc::termios) -> Self {
        if (ios.c_cflag & libc::PARENB) == 0 {
            Parity::None
        } else if (ios.c_cflag & libc::PARODD) == 0 {
            Parity::Even
        } else {
            Parity::Odd
        }
    }

    fn store(self, ios: &mut libc::termios, soc: &Socket) -> Result<()> {
        match self {
            Parity::None => {
                ios.c_iflag |= libc::IGNPAR;
                ios.c_cflag &= !(libc::PARENB | libc::PARODD);
            }
            Parity::Even => {
                ios.c_iflag &= !(libc::IGNPAR | libc::PARMRK);
                ios.c_iflag |= libc::INPCK;
                ios.c_cflag |= libc::PARENB;
                ios.c_cflag &= !libc::PARODD;
            }
            Parity::Odd => {
                ios.c_iflag &= !(libc::IGNPAR | libc::PARMRK);
                ios.c_iflag |= libc::INPCK;
                ios.c_cflag |= libc::PARENB | libc::PARODD;
            }
        }
        unsafe {
            match libc::tcsetattr(soc.0.as_raw_fd(), libc::TCSANOW, ios) {
                -1 => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }
}

impl SerialPortOpt for StopBits {
    fn load(ios: &libc::termios) -> Self {
        if (ios.c_cflag & libc::CSTOPB) == 0 {
            StopBits::One
        } else {
            StopBits::Two
        }
    }

    fn store(self, ios: &mut libc::termios, soc: &Socket) -> Result<()> {
        match self {
            StopBits::One => ios.c_cflag &= !libc::CSTOPB,
            StopBits::Two => ios.c_cflag |= libc::CSTOPB,
        }
        unsafe {
            match libc::tcsetattr(soc.0.as_raw_fd(), libc::TCSANOW, ios) {
                -1 => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }
}

pub struct SerialPort {
    ctx: IoContext,
    soc: Socket,
    ios: libc::termios,
    t: Timeout,
}

impl SerialPort {
    pub fn open(ctx: &IoContext, device: &CStr) -> Result<SerialPort> {
        let fd = Fd::open(device)?;
        let ios = setup_termios(&fd)?;
        Ok(SerialPort {
            ctx: ctx.clone(),
            soc: Socket(fd),
            ios: ios,
            t: Timeout::INFINITE,
        })
    }

    pub fn close(self) -> Result<()> {
        self.soc.close()
    }

    pub fn get_option<S>(&self) -> S
    where
        S: SerialPortOpt,
    {
        S::load(&self.ios)
    }

    pub fn nb_read_some(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.nb_read(buf)
    }

    pub fn nb_write_some(&self, buf: &[u8]) -> Result<usize> {
        self.soc.nb_write(buf)
    }

    pub fn read_some(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.read(&self.ctx, buf, self.t)
    }

    pub fn send_break(&self) -> Result<()> {
        tcsendbreak(&self.soc.0, 0)
    }

    pub fn set_option<S>(&mut self, opt: S) -> Result<()>
    where
        S: SerialPortOpt,
    {
        opt.store(&mut self.ios, &self.soc)
    }

    pub const fn set_timeout(&mut self, timeout: Duration) {
        self.t = Timeout::from_duration(timeout)
    }

    pub fn write_some(&self, buf: &[u8]) -> Result<usize> {
        self.soc.write(&self.ctx, buf, self.t)
    }
}

pub struct AsyncSerialPort {
    soc: AsyncSocket<()>,
    ios: libc::termios,
    t: Timeout,
}

impl AsyncSerialPort {
    pub fn as_ctx(&self) -> &IoContext {
        &self.soc.as_ctx()
    }

    pub fn nb_read_some(&self, buf: &mut [u8]) -> std::result::Result<usize, OsError> {
        self.soc.as_socket().nb_read(buf)
    }

    pub fn nb_write_some(&self, buf: &[u8]) -> std::result::Result<usize, OsError> {
        self.soc.as_socket().nb_write(buf)
    }

    pub fn get_option<S>(&self) -> S
    where
        S: SerialPortOpt,
    {
        S::load(&self.ios)
    }

    pub fn read_some(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.as_socket().read(self.soc.as_ctx(), buf, self.t)
    }

    pub fn send_break(&self) -> Result<()> {
        tcsendbreak(&self.soc.as_socket().0, 0)
    }

    pub fn set_option<S>(&mut self, opt: S) -> Result<()>
    where
        S: SerialPortOpt,
    {
        opt.store(&mut self.ios, self.soc.as_socket())
    }

    pub const fn set_timeout(&mut self, timeout: Duration) {
        self.t = Timeout::from_duration(timeout)
    }

    pub fn write_some(&self, buf: &[u8]) -> Result<usize> {
        self.soc.as_socket().write(self.as_ctx(), buf, self.t)
    }

    pub async fn async_read_some(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.async_read(buf, self.t).await
    }

    pub async fn async_write_some(&self, buf: &[u8]) -> Result<usize> {
        self.soc.async_write(buf, self.t).await
    }
}

impl From<SerialPort> for AsyncSerialPort {
    fn from(soc: SerialPort) -> AsyncSerialPort {
        Self {
            soc: AsyncSocket::new(soc.ctx, soc.soc, ()),
            ios: soc.ios,
            t: soc.t,
        }
    }
}
