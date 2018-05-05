use core::{Protocol, Endpoint, Socket, IoControl, GetSocketOption, SetSocketOption};

use libc;
use std::io;
use std::mem;
use std::ptr;
use std::fmt;
use std::cell::Cell;
use std::ffi::CStr;
use std::time::Duration;
use errno::{errno, Errno};

pub use std::os::unix::io::{AsRawFd, RawFd};
pub use libc::{addrinfo, c_void, in_addr, ip_mreq, linger, sockaddr, sockaddr_in,
               sockaddr_storage, sockaddr_un, socklen_t, AF_INET6, IPPROTO_IPV6,
               IPV6_MULTICAST_LOOP, IPV6_V6ONLY, in6_addr, ipv6_mreq, sockaddr_in6, AF_INET,
               AF_UNIX, FD_CLOEXEC, FD_SETSIZE, FIONBIO, F_GETFD, F_GETFL, F_SETFD, F_SETFL,
               IPPROTO_IP, IPPROTO_TCP, IP_ADD_MEMBERSHIP, IP_DROP_MEMBERSHIP, IP_MULTICAST_LOOP,
               IP_MULTICAST_TTL, IP_TTL, O_CLOEXEC, O_NONBLOCK, SOCK_DGRAM, SOCK_RAW,
               SOCK_SEQPACKET, SOCK_STREAM, SOL_SOCKET, SO_BROADCAST, SO_DEBUG, SO_DONTROUTE,
               SO_ERROR, SO_KEEPALIVE, SO_LINGER, SO_RCVBUF, SO_RCVLOWAT, SO_REUSEADDR, SO_SNDBUF,
               SO_SNDLOWAT, TCP_NODELAY, FIONREAD};
#[cfg(target_os = "linux")]
pub use libc::{SOCK_CLOEXEC, SOCK_NONBLOCK};

pub const IPV6_UNICAST_HOPS: libc::c_int = 16;
pub const IPV6_MULTICAST_IF: libc::c_int = 17;
pub const IPV6_MULTICAST_HOPS: libc::c_int = 18;
pub const IP_MULTICAST_IF: libc::c_int = 32;
pub const IPPROTO_ICMP: libc::c_int = 1;
pub const IPPROTO_ICMPV6: libc::c_int = 58;
pub const IPPROTO_UDP: libc::c_int = 17;
pub const AF_UNSPEC: libc::c_int = 0;
pub const AI_PASSIVE: libc::c_int = 0x0001;
#[allow(dead_code)]
pub const AI_NUMERICHOST: libc::c_int = 0x0004;
pub const AI_NUMERICSERV: libc::c_int = 0x0400;

#[cfg(target_os = "linux")]
pub const IPV6_JOIN_GROUP: libc::c_int = 20;
#[cfg(target_os = "linux")]
pub const IPV6_LEAVE_GROUP: libc::c_int = 21;
#[cfg(target_os = "macos")]
pub use libc::{IPV6_JOIN_GROUP, IPV6_LEAVE_GROUP};

/// A list specifying POSIX categories of signal.
#[repr(i32)]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum Signal {
    /// Hangup detected on controlling terminal or death of controlling process.
    SIGHUP = libc::SIGHUP,

    /// Interrupt from keyboard.
    SIGINT = libc::SIGINT,

    /// Quit from keyboard.
    SIGQUIT = libc::SIGQUIT,

    /// Illegal Instruction.
    SIGILL = libc::SIGILL,

    /// Abort signal from abort(3)
    SIGABRT = libc::SIGABRT,

    /// Floating point exception.
    SIGFPE = libc::SIGFPE,

    /// Kill signal.
    SIGKILL = libc::SIGKILL,

    /// Invalid memory reference.
    SIGSEGV = libc::SIGSEGV,

    /// Broken pipe: write to pipe with no readers.
    SIGPIPE = libc::SIGPIPE,

    /// Timer signal from alarm(2).
    SIGALRM = libc::SIGALRM,

    /// Termination signal.
    SIGTERM = libc::SIGTERM,

    /// User-defined signal 1.
    SIGUSR1 = libc::SIGUSR1,

    /// User-defined signal 2.
    SIGUSR2 = libc::SIGUSR2,

    /// Child stopped of terminated.
    SIGCHLD = libc::SIGCHLD,

    /// Continue if stopped.
    SIGCONT = libc::SIGCONT,

    /// Stop process.
    SIGSTOP = libc::SIGSTOP,

    /// Stop typed at terminal.
    SIGTSTP = libc::SIGTSTP,

    /// Terminal input for background process.
    SIGTTIN = libc::SIGTTIN,

    /// Terminal output for background process.
    SIGTTOU = libc::SIGTTOU,

    /// Bus error (bad memory access).
    SIGBUS = libc::SIGBUS,

    /// Pollable event (Sys V). Synonym for SIGIO.
    #[cfg(target_os = "linux")]
    SIGPOLL = libc::SIGPOLL,

    /// Profiling timer expired.
    SIGPROF = libc::SIGPROF,

    /// Bad argument to routine (SVr4).
    SIGSYS = libc::SIGSYS,

    /// Trace/breakpoint trap.
    SIGTRAP = libc::SIGTRAP,

    /// Urgent condition on socket (4.2BSD).
    SIGURG = libc::SIGURG,

    /// Virtual alarm clock (4.2BSD).
    SIGVTALRM = libc::SIGVTALRM,

    /// CPU time limit exceeded (4.2BSD).
    SIGXCPU = libc::SIGXCPU,

    /// File size limit exceeded (4.2BSD).
    SIGXFSZ = libc::SIGXFSZ,
}

#[cfg(target_os = "linux")]
pub fn raise(sig: Signal) -> Result<(), SystemError> {
    match unsafe { libc::raise(sig as i32) } {
        -1 => Err(SystemError::last_error()),
        _ => Ok(()),
    }
}

#[cfg(target_os = "macos")]
pub fn raise(sig: Signal) -> Result<(), SystemError> {
    match unsafe { libc::kill(libc::getpid(), sig as i32) } {
        -1 => Err(SystemError::last_error()),
        _ => Ok(()),
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct SystemError(Errno);

impl SystemError {
    pub fn last_error() -> Self {
        SystemError(errno())
    }

    #[cfg(target_os = "macos")]
    pub fn from_signal(sig: Signal) -> Self {
        SystemError(Errno(-(sig as i32)))
    }

    #[cfg(target_os = "macos")]
    pub fn try_signal(self) -> Result<Signal, Self> {
        if (self.0).0 < 0 {
            Ok(unsafe { mem::transmute(-(self.0).0) })
        } else {
            Err(self)
        }
    }
}

impl Default for SystemError {
    fn default() -> Self {
        SystemError(Errno(0))
    }
}

impl fmt::Display for SystemError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<SystemError> for io::Error {
    fn from(err: SystemError) -> Self {
        io::Error::from_raw_os_error((err.0).0)
    }
}

// /// Permission denied.
// pub const ACCESS_DENIED: SystemError = SystemError(Errno(libc::EACCES));

/// Address family not supported by protocol.
pub const ADDRESS_FAMILY_NOT_SUPPORTED: SystemError = SystemError(Errno(libc::EAFNOSUPPORT));

// /// Address already in use.
// pub const ADDRESS_IN_USE: SystemError = SystemError(Errno(libc::EADDRINUSE));

// /// Transport endpoint is already connected.
// pub const ALREADY_CONNECTED: SystemError = SystemError(Errno(libc::EISCONN));

// /// Operation already in progress.
// pub const ALREADY_STARTED: SystemError = SystemError(Errno(libc::EALREADY));

// /// Broken pipe.
// pub const BROKEN_PIPE: SystemError = SystemError(Errno(libc::EPIPE));

/// A connection has been aborted.
pub const CONNECTION_ABORTED: SystemError = SystemError(Errno(libc::ECONNABORTED));

// /// connection refused.
// pub const CONNECTION_REFUSED: SystemError = SystemError(Errno(libc::ECONNREFUSED));

// /// Connection reset by peer.
// pub const CONNECTION_RESET: SystemError = SystemError(Errno(libc::ECONNRESET));

// /// Bad file descriptor.
// pub const BAD_DESCRIPTOR: SystemError = SystemError(Errno(libc::EBADF));

// /// Bad address.
// pub const FAULT: SystemError = SystemError(Errno(libc::EFAULT));

// /// No route to host.
// pub const HOST_UNREACHABLE: SystemError = SystemError(Errno(libc::EHOSTUNREACH));

/// peration now in progress.
pub const IN_PROGRESS: SystemError = SystemError(Errno(libc::EINPROGRESS));

/// Interrupted system call.
pub const INTERRUPTED: SystemError = SystemError(Errno(libc::EINTR));

/// Invalid argument.
pub const INVALID_ARGUMENT: SystemError = SystemError(Errno(libc::EINVAL));

// /// Message to long.
// pub const MESSAGE_SIZE: SystemError = SystemError(Errno(libc::EMSGSIZE));

/// The name was too long.
pub const NAME_TOO_LONG: SystemError = SystemError(Errno(libc::ENAMETOOLONG));

// /// Network is down.
// pub const NETWORK_DOWN: SystemError = SystemError(Errno(libc::ENETDOWN));

// /// Network dropped connection on reset.
// pub const NETWORK_RESET: SystemError = SystemError(Errno(libc::ENETRESET));

// /// Network is unreachable.
// pub const NETWORK_UNREACHABLE: SystemError = SystemError(Errno(libc::ENETUNREACH));

// /// Too many open files.
// pub const NO_DESCRIPTORS: SystemError = SystemError(Errno(libc::EMFILE));

/// No buffer space available.
pub const NO_BUFFER_SPACE: SystemError = SystemError(Errno(libc::ENOBUFS));

// /// Cannot allocate memory.
// pub const NO_MEMORY: SystemError = SystemError(Errno(libc::ENOMEM));

// /// Operation not permitted.
// pub const NO_PERMISSION: SystemError = SystemError(Errno(libc::EPERM));

// /// Protocol not available.
// pub const NO_PROTOCOL_OPTION: SystemError = SystemError(Errno(libc::ENOPROTOOPT));

// /// No such device.
// pub const NO_SUCH_DEVICE: SystemError = SystemError(Errno(libc::ENODEV));

// /// Transport endpoint is not connected.
// pub const NOT_CONNECTED: SystemError = SystemError(Errno(libc::ENOTCONN));

// /// Socket operation on non-socket.
// pub const NOT_SOCKET: SystemError = SystemError(Errno(libc::ENOTSOCK));

/// Operation cancelled.
pub const OPERATION_CANCELED: SystemError = SystemError(Errno(libc::ECANCELED));

// /// Operation not supported.
// pub const OPERATION_NOT_SUPPORTED: SystemError = SystemError(Errno(libc::EOPNOTSUPP));

// /// Cannot send after transport endpoint shutdown.
// pub const SHUT_DOWN: SystemError = SystemError(Errno(libc::ESHUTDOWN));

/// Connection timed out.
pub const TIMED_OUT: SystemError = SystemError(Errno(libc::ETIMEDOUT));

/// Resource temporarily unavailable.
pub const TRY_AGAIN: SystemError = SystemError(Errno(libc::EAGAIN));

/// The socket is marked non-blocking and the requested operation would block.
pub const WOULD_BLOCK: SystemError = SystemError(Errno(libc::EWOULDBLOCK));

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct AddrinfoError(i32);

impl fmt::Display for AddrinfoError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            unsafe { CStr::from_ptr(libc::gai_strerror(self.0)) }
                .to_str()
                .unwrap()
        )
    }
}

impl From<AddrinfoError> for io::Error {
    fn from(err: AddrinfoError) -> Self {
        io::Error::new(io::ErrorKind::Other, format!("{}", err))
    }
}

/// The service is not supported for the given socket type.
pub const SERVICE_NOT_FOUND: AddrinfoError = AddrinfoError(EAI_SERVICE);
const EAI_SERVICE: i32 = 9;

// /// The socket type is not supported.
// pub const SOCKET_TYPE_NOT_SUPPORTED: AddrinfoError = AddrinfoError(EAI_SOCKTYPE);
// const EAI_SOCKTYPE: i32 = 10;

/// Possible values which can be passed to the shutdown method.
#[repr(i32)]
pub enum Shutdown {
    /// Indicates that the reading portion of this socket should be shut down.
    Read = libc::SHUT_RD,

    /// Indicates that the writing portion of this socket should be shut down.
    Write = libc::SHUT_WR,

    /// Shut down both the reading and writing portions of this socket.
    Both = libc::SHUT_RDWR,
}

pub struct Timeout {
    nano_sec: Cell<Duration>,
    milli_sec: Cell<i32>,
}

const TIMEOUT_MAX: u64 = 60 * 60 * 2; // 2h

impl Timeout {
    pub fn max() -> Self {
        Timeout {
            nano_sec: Cell::new(Duration::new(TIMEOUT_MAX as u64, 0)),
            milli_sec: Cell::new(TIMEOUT_MAX as i32 * 1000),
        }
    }

    pub fn get(&self) -> Duration {
        self.nano_sec.get()
    }

    pub fn set(&self, nano_sec: Duration) -> Result<(), SystemError> {
        if nano_sec.as_secs() >= TIMEOUT_MAX {
            Err(INVALID_ARGUMENT)
        } else {
            self.nano_sec.set(nano_sec);
            self.milli_sec.set(
                (nano_sec.as_secs() * 1000 / nano_sec.subsec_nanos() as u64 /
                    1000000) as i32,
            );
            Ok(())
        }
    }

    pub fn milliseconds(&self) -> i32 {
        self.milli_sec.get()
    }
}

#[cfg(target_os = "macos")]
fn init_fd(fd: RawFd) {
    unsafe {
        // FD_CLOEXEC
        let flags = libc::fcntl(fd, F_GETFD);
        assert!(flags != -1, "{}", errno());
        let flags = libc::fcntl(fd, F_SETFD, flags | FD_CLOEXEC);
        assert!(flags != -1, "{}", errno());

        // O_NONBLOCK
        let flags = libc::fcntl(fd, F_GETFL);
        assert!(flags != -1, "{}", errno());
        let flags = libc::fcntl(fd, F_SETFL, flags | O_NONBLOCK);
        assert!(flags != -1, "{}", errno());
    }
}

#[cfg(target_os = "macos")]
pub fn accept<P, S>(soc: &S) -> Result<(RawFd, P::Endpoint), SystemError>
where
    P: Protocol,
    S: Socket<P>,
{
    let mut sa = unsafe { soc.protocol().uninitialized() };
    let mut salen = sa.capacity();
    match unsafe { libc::accept(soc.as_raw_fd(), sa.as_mut_ptr(), &mut salen) } {
        -1 => Err(SystemError::last_error()),
        fd => unsafe {
            init_fd(fd);
            sa.resize(salen);
            Ok((fd, sa))
        },
    }
}

#[cfg(target_os = "linux")]
pub fn accept<P, S>(soc: &S) -> Result<(RawFd, P::Endpoint), SystemError>
where
    P: Protocol,
    S: Socket<P>,
{
    let mut sa = unsafe { soc.protocol().uninitialized() };
    let mut salen = sa.capacity();
    match unsafe {
        libc::accept4(
            soc.as_raw_fd(),
            sa.as_mut_ptr(),
            &mut salen,
            SOCK_NONBLOCK | SOCK_CLOEXEC,
        )
    } {
        -1 => Err(SystemError::last_error()),
        fd => unsafe {
            sa.resize(salen);
            Ok((fd, sa))
        },
    }
}

pub fn bind<P, S>(soc: &S, sa: &P::Endpoint) -> Result<(), SystemError>
where
    P: Protocol,
    S: Socket<P>,
{
    match unsafe { libc::bind(soc.as_raw_fd(), sa.as_ptr(), sa.size()) } {
        -1 => Err(SystemError::last_error()),
        _ => Ok(()),
    }
}

#[cfg(debug_assertions)]
pub fn close(fd: RawFd) {
    if 0 != unsafe { libc::close(fd) } {
        panic!("{}", SystemError::last_error());
    }
}

#[cfg(not(debug_assertions))]
pub fn close(fd: RawFd) {
    unsafe { libc::close(fd) };
}

pub fn connect<P, S>(soc: &S, sa: &P::Endpoint) -> Result<(), SystemError>
where
    P: Protocol,
    S: Socket<P>,
{
    match unsafe { libc::connect(soc.as_raw_fd(), sa.as_ptr(), sa.size()) } {
        -1 => Err(SystemError::last_error()),
        _ => Ok(()),
    }
}

#[cfg(not(target_os = "macos"))]
pub fn connection_check<P, S>(_: &S) -> Result<(), SystemError>
where
    P: Protocol,
    S: Socket<P>,
{
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn connection_check<P, S>(soc: &S) -> Result<(), SystemError>
where
    P: Protocol,
    S: Socket<P>,
{
    let mut buf = [1; 0];
    ::std::thread::sleep(::std::time::Duration::new(1, 0));
    match unsafe { libc::read(soc.as_raw_fd(), buf.as_mut_ptr() as *mut _, 0) } {
        -1 => Err(SystemError::last_error()),
        len => {
            println!("{}", len);
            Ok(())
        }
    }
}

pub fn freeaddrinfo(ai: *mut addrinfo) {
    unsafe { libc::freeaddrinfo(ai) }
}

pub fn getaddrinfo<P>(
    pro: &P,
    node: &CStr,
    serv: &CStr,
    flags: i32,
) -> Result<*mut addrinfo, AddrinfoError>
where
    P: Protocol,
{
    let mut hints: addrinfo = unsafe { mem::zeroed() };
    hints.ai_flags = flags;
    hints.ai_family = pro.family_type();
    hints.ai_socktype = pro.socket_type();
    hints.ai_protocol = pro.protocol_type();

    let node = if node.to_bytes().is_empty() {
        ptr::null()
    } else {
        node.as_ptr()
    };

    let serv = if serv.to_bytes().is_empty() {
        ptr::null()
    } else {
        serv.as_ptr()
    };

    let mut base: *mut addrinfo = ptr::null_mut();
    match unsafe { libc::getaddrinfo(node, serv, &hints, &mut base) } {
        0 => Ok(base),
        ec => Err(AddrinfoError(ec)),
    }
}

pub fn gethostname() -> Result<String, SystemError> {
    let mut name: [libc::c_char; 65] = unsafe { mem::uninitialized() };
    match unsafe { libc::gethostname(name.as_mut_ptr(), mem::size_of_val(&name)) } {
        -1 => Err(SystemError::last_error()),
        _ => unsafe {
            let cstr = CStr::from_ptr(name.as_ptr());
            Ok(String::from(cstr.to_str().unwrap()))
        },
    }
}

pub fn getpeername<P, S>(soc: &S) -> Result<P::Endpoint, SystemError>
where
    P: Protocol,
    S: Socket<P>,
{
    let mut sa = unsafe { soc.protocol().uninitialized() };
    let mut salen = sa.capacity();
    match unsafe { libc::getpeername(soc.as_raw_fd(), sa.as_mut_ptr(), &mut salen) } {
        -1 => Err(SystemError::last_error()),
        _ => unsafe {
            sa.resize(salen);
            Ok(sa)
        },
    }
}

pub fn getsockname<P, S>(soc: &S) -> Result<P::Endpoint, SystemError>
where
    P: Protocol,
    S: Socket<P>,
{
    let mut sa = unsafe { soc.protocol().uninitialized() };
    let mut salen = sa.capacity();
    match unsafe { libc::getsockname(soc.as_raw_fd(), sa.as_mut_ptr(), &mut salen) } {
        -1 => Err(SystemError::last_error()),
        _ => unsafe {
            sa.resize(salen);
            Ok(sa)
        },
    }
}

pub fn getsockopt<P, S, D>(soc: &S) -> Result<D, SystemError>
where
    P: Protocol,
    S: Socket<P>,
    D: GetSocketOption<P>,
{
    let pro = soc.protocol();
    let mut data = D::default();
    let mut datalen = data.capacity();
    match unsafe {
        libc::getsockopt(
            soc.as_raw_fd(),
            data.level(pro),
            data.name(pro),
            data.as_mut_ptr(),
            &mut datalen,
        )
    } {
        -1 => Err(SystemError::last_error()),
        _ => unsafe {
            data.resize(datalen);
            Ok(data)
        },
    }
}

pub fn if_nametoindex(name: &CStr) -> Result<u32, SystemError> {
    match unsafe { libc::if_nametoindex(name.as_ptr()) } {
        0 => Err(SystemError::last_error()),
        ifi => Ok(ifi),
    }
}

pub fn ioctl<S, D>(soc: &S, data: &mut D) -> Result<(), SystemError>
where
    S: AsRawFd,
    D: IoControl,
{
    match unsafe { libc::ioctl(soc.as_raw_fd(), data.name(), data.as_mut_ptr()) } {
        -1 => Err(SystemError::last_error()),
        _ => Ok(()),
    }
}

pub fn listen<P, S>(soc: &S, backlog: i32) -> Result<(), SystemError>
where
    P: Protocol,
    S: Socket<P>,
{
    match unsafe { libc::listen(soc.as_raw_fd(), backlog) } {
        -1 => Err(SystemError::last_error()),
        _ => Ok(()),
    }
}

#[cfg(target_os = "macos")]
pub fn pipe() -> Result<(RawFd, RawFd), SystemError> {
    let mut fds: [RawFd; 2] = unsafe { mem::uninitialized() };
    match unsafe { libc::pipe(fds.as_mut_ptr()) } {
        -1 => Err(SystemError::last_error()),
        _ => {
            init_fd(fds[0]);
            init_fd(fds[1]);
            Ok((fds[0], fds[1]))
        }
    }
}

#[cfg(target_os = "linux")]
#[allow(dead_code)]
pub fn pipe() -> Result<(RawFd, RawFd), SystemError> {
    let mut fds: [RawFd; 2] = unsafe { mem::uninitialized() };
    match unsafe { libc::pipe2(fds.as_mut_ptr(), O_CLOEXEC | O_NONBLOCK) } {
        -1 => Err(SystemError::last_error()),
        _ => Ok((fds[0], fds[1])),
    }
}

pub fn read<S>(soc: &S, buf: &mut [u8]) -> Result<usize, SystemError>
where
    S: AsRawFd,
{
    debug_assert!(buf.len() > 0);
    match unsafe { libc::read(soc.as_raw_fd(), buf.as_mut_ptr() as *mut _, buf.len()) } {
        -1 => Err(SystemError::last_error()),
        0 => Err(CONNECTION_ABORTED),
        len => Ok(len as usize),
    }
}

pub fn readable<S>(soc: &S, timeout: &Timeout) -> Result<(), SystemError>
where
    S: AsRawFd,
{
    let mut pfd: libc::pollfd = unsafe { mem::uninitialized() };
    pfd.fd = soc.as_raw_fd();
    pfd.events = libc::POLLIN;

    match unsafe { libc::poll(&mut pfd, 1, timeout.milliseconds()) } {
        0 => Err(TIMED_OUT),
        -1 => Err(SystemError::last_error()),
        _ => Ok(()),
    }
}

pub fn recv<P, S>(soc: &S, buf: &mut [u8], flags: i32) -> Result<usize, SystemError>
where
    P: Protocol,
    S: Socket<P>,
{
    debug_assert!(buf.len() > 0);
    match unsafe {
        libc::recv(
            soc.as_raw_fd(),
            buf.as_mut_ptr() as *mut _,
            buf.len(),
            flags,
        )
    } {
        -1 => Err(SystemError::last_error()),
        0 => Err(CONNECTION_ABORTED),
        len => Ok(len as usize),
    }
}

pub fn recvfrom<P, S>(
    soc: &S,
    buf: &mut [u8],
    flags: i32,
) -> Result<(usize, P::Endpoint), SystemError>
where
    P: Protocol,
    S: Socket<P>,
{
    debug_assert!(buf.len() > 0);
    let mut sa = unsafe { soc.protocol().uninitialized() };
    let mut salen = sa.capacity();
    match unsafe {
        libc::recvfrom(
            soc.as_raw_fd(),
            buf.as_mut_ptr() as *mut _,
            buf.len(),
            flags,
            sa.as_mut_ptr(),
            &mut salen,
        )
    } {
        -1 => Err(SystemError::last_error()),
        0 => Err(CONNECTION_ABORTED),
        len => unsafe {
            sa.resize(salen);
            Ok((len as usize, sa))
        },
    }
}

pub fn setsockopt<P, S, D>(soc: &S, data: D) -> Result<(), SystemError>
where
    P: Protocol,
    S: Socket<P>,
    D: SetSocketOption<P>,
{
    let pro = soc.protocol();
    match unsafe {
        libc::setsockopt(
            soc.as_raw_fd(),
            data.level(pro),
            data.name(pro),
            data.as_ptr(),
            data.size(),
        )
    } {
        -1 => Err(SystemError::last_error()),
        _ => Ok(()),
    }
}

pub fn send<P, S>(soc: &S, buf: &[u8], flags: i32) -> Result<usize, SystemError>
where
    P: Protocol,
    S: Socket<P>,
{
    debug_assert!(buf.len() > 0);
    match unsafe { libc::send(soc.as_raw_fd(), buf.as_ptr() as *const _, buf.len(), flags) } {
        -1 => Err(SystemError::last_error()),
        0 => Err(CONNECTION_ABORTED),
        len => Ok(len as usize),
    }
}

pub fn sendto<P, S>(soc: &S, buf: &[u8], flags: i32, sa: &P::Endpoint) -> Result<usize, SystemError>
where
    P: Protocol,
    S: Socket<P>,
{
    debug_assert!(buf.len() > 0);
    match unsafe {
        libc::sendto(
            soc.as_raw_fd(),
            buf.as_ptr() as *const _,
            buf.len(),
            flags,
            sa.as_ptr(),
            sa.size(),
        )
    } {
        -1 => Err(SystemError::last_error()),
        0 => Err(CONNECTION_ABORTED),
        len => Ok(len as usize),
    }
}

pub fn shutdown<P, S>(soc: &S, how: Shutdown) -> Result<(), SystemError>
where
    P: Protocol,
    S: Socket<P>,
{
    match unsafe { libc::shutdown(soc.as_raw_fd(), how as i32) } {
        -1 => Err(SystemError::last_error()),
        _ => Ok(()),
    }
}

pub fn sock_error<S>(soc: &S) -> SystemError
where
    S: AsRawFd,
{
    let mut err: i32 = 0;
    let mut errlen = 4;
    unsafe {
        libc::getsockopt(
            soc.as_raw_fd(),
            SOL_SOCKET,
            SO_ERROR,
            &mut err as *mut _ as *mut libc::c_void,
            &mut errlen,
        )
    };
    SystemError(Errno(err))
}

#[cfg(target_os = "macos")]
pub fn socket<P>(pro: &P) -> Result<RawFd, SystemError>
where
    P: Protocol,
{
    match unsafe { libc::socket(pro.family_type(), pro.socket_type(), pro.protocol_type()) } {
        -1 => Err(SystemError::last_error()),
        fd => {
            init_fd(fd);
            Ok(fd)
        }
    }
}

#[cfg(target_os = "linux")]
pub fn socket<P>(pro: &P) -> Result<RawFd, SystemError>
where
    P: Protocol,
{
    match unsafe {
        libc::socket(
            pro.family_type(),
            pro.socket_type() | SOCK_NONBLOCK | SOCK_CLOEXEC,
            pro.protocol_type(),
        )
    } {
        -1 => Err(SystemError::last_error()),
        fd => Ok(fd),
    }
}

#[cfg(target_os = "macos")]
pub fn socketpair<P>(pro: &P) -> Result<(RawFd, RawFd), SystemError>
where
    P: Protocol,
{
    let mut fds: [RawFd; 2] = unsafe { mem::uninitialized() };
    match unsafe {
        libc::socketpair(
            pro.family_type(),
            pro.socket_type(),
            pro.protocol_type(),
            fds.as_mut_ptr(),
        )
    } {
        -1 => Err(SystemError::last_error()),
        _ => {
            init_fd(fds[0]);
            init_fd(fds[1]);
            Ok((fds[0], fds[1]))
        }
    }
}

#[cfg(target_os = "linux")]
pub fn socketpair<P>(pro: &P) -> Result<(RawFd, RawFd), SystemError>
where
    P: Protocol,
{
    let mut fds: [RawFd; 2] = unsafe { mem::uninitialized() };
    match unsafe {
        libc::socketpair(
            pro.family_type(),
            pro.socket_type() | SOCK_NONBLOCK | SOCK_CLOEXEC,
            pro.protocol_type(),
            fds.as_mut_ptr(),
        )
    } {
        -1 => Err(SystemError::last_error()),
        _ => Ok((fds[0], fds[1])),
    }
}

pub fn write<S>(soc: &S, buf: &[u8]) -> Result<usize, SystemError>
where
    S: AsRawFd,
{
    debug_assert!(buf.len() > 0);
    match unsafe { libc::write(soc.as_raw_fd(), buf.as_ptr() as *const _, buf.len()) } {
        -1 => Err(SystemError::last_error()),
        len => Ok(len as usize),
    }
}

pub fn writable<S>(soc: &S, timeout: &Timeout) -> Result<(), SystemError>
where
    S: AsRawFd,
{
    let mut pfd: libc::pollfd = unsafe { mem::uninitialized() };
    pfd.fd = soc.as_raw_fd();
    pfd.events = libc::POLLOUT;

    match unsafe { libc::poll(&mut pfd, 1, timeout.milliseconds()) } {
        0 => Err(TIMED_OUT),
        -1 => Err(SystemError::last_error()),
        _ => Ok(()),
    }
}
