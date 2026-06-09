use super::Timeout;
use crate::buffer::MsgBuf;
use crate::error::{OsError, Result};
use crate::sockaddr::{SockAddr, SockLen};
use crate::socket_base::{Endpoint, EndpointRef, GetSockOpt, Protocol, SetSockOpt, Shutdown};
use std::ffi::CStr;
use std::mem::MaybeUninit;
use std::num::NonZero;
use std::{mem, ptr};

/// A list specifying POSIX categories of signal.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct Signal(NonZero<libc::c_int>);

impl Signal {
    /// Hangup detected on controlling terminal or death of controlling process.
    pub const HUP: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGHUP) });

    /// Interrupt from keyboard.
    pub const INT: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGINT) });

    /// Quit from keyboard.
    pub const QUIT: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGQUIT) });

    /// Illegal Instruction.
    pub const ILL: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGILL) });

    /// Abort signal from abort(3)
    pub const ABRT: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGABRT) });

    /// Floating point exception.
    pub const FPE: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGFPE) });

    /// Kill signal.
    pub const KILL: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGKILL) });

    /// Invalid memory reference.
    pub const SEGV: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGSEGV) });

    /// Broken pipe: write to pipe with no readers.
    pub const PIPE: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGPIPE) });

    /// Timer signal from alarm(2).
    pub const ALRM: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGALRM) });

    /// Termination signal.
    pub const TERM: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGTERM) });

    /// User-defined signal 1.
    pub const USR1: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGUSR1) });

    /// User-defined signal 2.
    pub const USR2: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGUSR2) });

    /// Child stopped of terminated.
    pub const CHLD: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGCHLD) });

    /// Continue if stopped.
    pub const CONT: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGCONT) });

    /// Stop process.
    pub const STOP: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGSTOP) });

    /// Stop typed at terminal.
    pub const TSTP: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGTSTP) });

    /// Terminal input for background process.
    pub const TTIN: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGTTIN) });

    /// Terminal output for background process.
    pub const TTOU: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGTTOU) });

    /// Bus error (bad memory access).
    pub const BUS: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGBUS) });

    /// Pollable event (Sys V). Synonym for SIGIO.
    #[cfg(target_os = "linux")]
    pub const POLL: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGPOLL) });

    /// Profiling timer expired.
    pub const PROF: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGPROF) });

    /// Bad argument to routine (SVr4).
    pub const SYS: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGSYS) });

    /// Trace/breakpoint trap.
    pub const TRAP: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGTRAP) });

    /// Urgent condition on socket (4.2BSD).
    pub const URG: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGURG) });

    /// Virtual alarm clock (4.2BSD).
    pub const VTALRM: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGVTALRM) });

    /// CPU time limit exceeded (4.2BSD).
    pub const XCPU: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGXCPU) });

    /// File size limit exceeded (4.2BSD).
    pub const XFSZ: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGXFSZ) });

    pub const fn number(&self) -> i32 {
        self.0.get()
    }

    #[cfg(target_os = "linux")]
    pub(crate) const unsafe fn from_signalfd_siginfo(ssi: &libc::signalfd_siginfo) -> Self {
        Self(unsafe { NonZero::new_unchecked(ssi.ssi_signo as i32) })
    }

    #[cfg(target_os = "macos")]
    pub(crate) unsafe fn from_kevent(kev: &libc::kevent) -> Self {
        Self(unsafe { NonZero::new_unchecked(kev.ident as libc::c_int) })
    }
}

pub(crate) struct Fd(libc::c_int);

impl Drop for Fd {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.0);
        }
    }
}

impl Fd {
    pub unsafe fn new_unchecked(fd: libc::c_int) -> Self {
        Self(fd)
    }

    pub const unsafe fn as_raw_fd(&self) -> libc::c_int {
        self.0
    }

    pub fn open(filename: &CStr) -> Result<Self> {
        unsafe {
            #[cfg(target_os = "linux")]
            let flags = libc::O_CLOEXEC | libc::O_NONBLOCK;
            #[cfg(target_os = "macos")]
            let flags = 0;
            match libc::open(filename.as_ptr(), flags) {
                -1 => Err(OsError::last()),
                soc => {
                    let fd = Self(soc);
                    #[cfg(target_os = "macos")]
                    fd.set_cloexec()?;
                    #[cfg(target_os = "macos")]
                    fd.set_nonblock()?;
                    Ok(fd)
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn set_cloexec(&self) -> Result<()> {
        unsafe {
            match libc::fcntl(self.0, libc::F_SETFD, libc::FD_CLOEXEC) {
                -1 => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    #[cfg(target_os = "macos")]
    fn set_nonblock(&self) -> Result<()> {
        unsafe {
            match libc::fcntl(self.0, libc::F_SETFL, libc::O_NONBLOCK) {
                -1 => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub fn read(&self, buf: &mut [u8]) -> Result<usize> {
        unsafe {
            match libc::read(self.0, buf.as_mut_ptr().cast(), buf.len()) {
                -1 => Err(OsError::last()),
                0 if !buf.is_empty() => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }

    pub fn write(&self, buf: &[u8]) -> Result<usize> {
        unsafe {
            match libc::write(self.0, buf.as_ptr().cast(), buf.len()) {
                -1 => Err(OsError::last()),
                0 if !buf.is_empty() => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }
}

/// Low-level UNIX-based socket type.
pub struct Socket(Fd);

#[cfg(doc)]
impl Drop for Socket {
    fn drop(&mut self) {}
}

impl Socket {
    pub(crate) unsafe fn from_raw_fd(fd: Fd) -> Self {
        Socket(fd)
    }

    pub fn new<P>(pro: P) -> Result<Self>
    where
        P: Protocol,
    {
        let socktype: i32 = pro.socket_type().into();
        #[cfg(target_os = "linux")]
        let socktype = socktype | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK;
        unsafe {
            match libc::socket(
                pro.family_type().into(),
                socktype,
                pro.protocol_type().into(),
            ) {
                -1 => Err(OsError::last()),
                soc => {
                    let soc = Fd::new_unchecked(soc);
                    #[cfg(target_os = "macos")]
                    soc.set_cloexec()?;
                    #[cfg(target_os = "macos")]
                    soc.set_nonblock()?;
                    Ok(Socket(soc))
                }
            }
        }
    }

    pub fn socketpair<P>(pro: P) -> Result<(Socket, Socket)>
    where
        P: Protocol,
    {
        let mut sv: [MaybeUninit<libc::c_int>; 2] = [const { MaybeUninit::uninit() }; 2];
        let socktype: i32 = pro.socket_type().into();
        #[cfg(target_os = "linux")]
        let socktype = socktype | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK;
        unsafe {
            match libc::socketpair(
                pro.family_type().into(),
                socktype,
                pro.protocol_type().into(),
                sv[0].as_mut_ptr(),
            ) {
                -1 => Err(OsError::last()),
                _ => {
                    let sv = mem::transmute::<_, [libc::c_int; 2]>(sv);
                    let s1 = Socket(Fd::new_unchecked(sv[0]));
                    let s2 = Socket(Fd::new_unchecked(sv[1]));
                    #[cfg(target_os = "macos")]
                    s1.0.set_cloexec()?;
                    #[cfg(target_os = "macos")]
                    s2.0.set_cloexec()?;
                    #[cfg(target_os = "macos")]
                    s1.0.set_nonblock()?;
                    #[cfg(target_os = "macos")]
                    s2.0.set_nonblock()?;
                    Ok((s1, s2))
                }
            }
        }
    }

    pub(crate) fn as_fd(&self) -> &Fd {
        &self.0
    }

    pub unsafe fn as_raw_fd(&self) -> libc::c_int {
        self.0.0
    }

    pub fn bind<E>(&self, ep: &EndpointRef<E>) -> Result<()>
    where
        E: Endpoint,
    {
        unsafe {
            let sa = ptr::from_ref(ep.sockaddr_ref()).cast();
            match libc::bind(self.0.0, sa, ep.sockaddr_len()) {
                -1 => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub fn listen(&self, backlog: i32) -> Result<()> {
        unsafe {
            match libc::listen(self.0.0, backlog) {
                -1 => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub fn connect<E>(&self, ep: &EndpointRef<E>) -> Result<()>
    where
        E: Endpoint,
    {
        unsafe {
            let sa = ptr::from_ref(ep.sockaddr_ref()).cast();
            match libc::connect(self.0.0, sa, ep.sockaddr_len()) {
                -1 => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub fn close(self) -> Result<()> {
        let Fd(fd) = self.0;
        unsafe {
            match libc::close(fd) {
                -1 => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub fn accept<E>(&self) -> Result<(Socket, E)>
    where
        E: Endpoint,
    {
        let mut sa = MaybeUninit::<E::SockAddr>::uninit();
        let mut sa_len = size_of::<E::SockAddr>() as SockLen;
        unsafe {
            #[cfg(target_os = "linux")]
            let res = libc::accept4(
                self.0.0,
                sa.as_mut_ptr().cast(),
                &mut sa_len,
                libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
            );
            #[cfg(target_os = "macos")]
            let res = libc::accept(self.0.0, sa.as_mut_ptr().cast(), &mut sa_len);
            match res {
                -1 => Err(OsError::last()),
                soc => {
                    let soc = Fd::new_unchecked(soc);
                    #[cfg(target_os = "macos")]
                    soc.set_cloexec()?;
                    #[cfg(target_os = "macos")]
                    soc.set_nonblock()?;
                    let ep = E::from_sockaddr(E::SockAddr::init(sa, sa_len));
                    Ok((Socket(soc), ep))
                }
            }
        }
    }

    pub fn receive(&self, buf: &mut [u8]) -> Result<usize> {
        unsafe {
            match libc::recv(self.0.0, buf.as_mut_ptr().cast(), buf.len(), 0) {
                -1 => Err(OsError::last()),
                0 => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }

    pub fn receive_from<E>(&self, buf: &mut [u8]) -> Result<(usize, E)>
    where
        E: Endpoint,
    {
        let mut sa = MaybeUninit::<E::SockAddr>::uninit();
        let mut sa_len = size_of::<E::SockAddr>() as SockLen;
        unsafe {
            match libc::recvfrom(
                self.0.0,
                buf.as_mut_ptr().cast(),
                buf.len(),
                0,
                sa.as_mut_ptr().cast(),
                &mut sa_len,
            ) {
                -1 => Err(OsError::last()),
                0 if !buf.is_empty() => Err(OsError::CONNECTION_ABORTED),
                len => Ok((
                    len as usize,
                    E::from_sockaddr(E::SockAddr::init(sa, sa_len)),
                )),
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn receive_msg_impl(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        unsafe {
            match libc::recvmsg(self.0.0, mbuf.as_ptr(), 0) {
                -1 => Err(OsError::last()),
                0 => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }

    #[cfg(target_os = "linux")]
    fn receive_msg_impl(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        if let Some(len) = mbuf.next() {
            Ok(len)
        } else {
            unsafe {
                mbuf.uninit();
                let mmsghdr = mbuf.as_mut_slice();
                match libc::recvmmsg(
                    self.0.0,
                    mmsghdr.as_mut_ptr(),
                    mmsghdr.len() as SockLen,
                    0,
                    ptr::null_mut(),
                ) {
                    -1 => Err(OsError::last()),
                    0 => Err(OsError::CONNECTION_ABORTED),
                    len => Ok(mbuf.set_len(len as usize)),
                }
            }
        }
    }

    pub fn receive_msg(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        self.receive_msg_impl(mbuf)
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize> {
        unsafe {
            match libc::send(self.0.0, buf.as_ptr().cast(), buf.len(), 0) {
                -1 => Err(OsError::last()),
                0 if !buf.is_empty() => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }

    pub fn send_to<E>(&self, buf: &[u8], ep: &EndpointRef<E>) -> Result<usize>
    where
        E: Endpoint,
    {
        unsafe {
            let sa = ptr::from_ref(ep.sockaddr_ref()).cast();
            match libc::sendto(
                self.0.0,
                buf.as_ptr().cast(),
                buf.len(),
                0,
                sa,
                ep.sockaddr_len() as SockLen,
            ) {
                -1 => Err(OsError::last()),
                0 if !buf.is_empty() => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn send_msg_impl(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        unsafe {
            match libc::sendmsg(self.0.0, mbuf.as_ptr(), 0) {
                -1 => Err(OsError::last()),
                0 => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }
    #[cfg(target_os = "linux")]
    fn send_msg_impl(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        if let Some(len) = mbuf.next() {
            Ok(len)
        } else {
            unsafe {
                let mmsghdr = mbuf.as_mut_slice();
                match libc::sendmmsg(self.0.0, mmsghdr.as_mut_ptr(), mmsghdr.len() as SockLen, 0) {
                    -1 => Err(OsError::last()),
                    0 => Err(OsError::CONNECTION_ABORTED),
                    len => Ok(mbuf.set_len(len as usize)),
                }
            }
        }
    }

    pub fn send_msg(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        self.send_msg_impl(mbuf)
    }

    pub fn read(&self, buf: &mut [u8]) -> Result<usize> {
        self.0.read(buf)
    }

    pub fn write(&self, buf: &[u8]) -> Result<usize> {
        self.0.write(buf)
    }

    pub fn getsockname<E>(&self) -> Result<E>
    where
        E: Endpoint,
    {
        let mut sa = MaybeUninit::<E::SockAddr>::uninit();
        let mut sa_len = size_of::<E::SockAddr>() as SockLen;
        unsafe {
            match libc::getsockname(self.0.0, sa.as_mut_ptr().cast(), &mut sa_len) {
                -1 => Err(OsError::last()),
                _ => Ok(E::from_sockaddr(E::SockAddr::init(sa, sa_len))),
            }
        }
    }

    pub fn getpeername<E>(&self) -> Result<E>
    where
        E: Endpoint,
    {
        let mut sa = MaybeUninit::<E::SockAddr>::uninit();
        let mut sa_len = size_of::<E::SockAddr>() as SockLen;
        unsafe {
            match libc::getpeername(self.0.0, sa.as_mut_ptr().cast(), &mut sa_len) {
                -1 => Err(OsError::last()),
                _ => Ok(E::from_sockaddr(E::SockAddr::init(sa, sa_len))),
            }
        }
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<()> {
        unsafe {
            match libc::shutdown(self.0.0, how as i32) {
                -1 => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub fn setsockopt<P>(&self, pro: P, opt: &dyn SetSockOpt<P>) -> Result<()>
    where
        P: Protocol,
    {
        let (key, data) = opt.data(pro);
        unsafe {
            match libc::setsockopt(
                self.0.0,
                key.level,
                key.name,
                data.as_ptr().cast(),
                data.len() as SockLen,
            ) {
                -1 => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub fn getsockopt<P, S>(&self, pro: P) -> Result<S>
    where
        P: Protocol,
        S: GetSockOpt<P>,
    {
        let (key, init) = S::init(pro);
        let mut data = MaybeUninit::<S>::uninit();
        let mut data_len = size_of::<S>() as SockLen;
        unsafe {
            match libc::getsockopt(
                self.0.0,
                key.level,
                key.name,
                data.as_mut_ptr().cast(),
                &mut data_len,
            ) {
                -1 => Err(OsError::last()),
                _ => Ok(init(data, data_len)),
            }
        }
    }

    pub fn poll_in(&self, timeout: Timeout) -> Result<()> {
        let mut poll = libc::pollfd {
            fd: self.0.0,
            events: libc::POLLIN,
            revents: 0,
        };
        unsafe {
            match libc::poll(&mut poll, 1, timeout.0) {
                -1 => Err(OsError::last()),
                0 => Err(OsError::OPERATION_CANCELED),
                _ => Ok(()),
            }
        }
    }

    pub fn poll_out(&self, timeout: Timeout) -> Result<()> {
        let mut poll = libc::pollfd {
            fd: self.0.0,
            events: libc::POLLOUT,
            revents: 0,
        };
        unsafe {
            match libc::poll(&mut poll, 1, timeout.0) {
                -1 => Err(OsError::last()),
                0 => Err(OsError::OPERATION_CANCELED),
                _ => Ok(()),
            }
        }
    }
}
