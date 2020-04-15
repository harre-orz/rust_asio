//

pub type NativeHandle = std::os::unix::io::RawFd;

use error::{ErrorCode, CONNECTION_ABORTED, INTERRUPTED, INVALID_ARGUMENT, SUCCESS, TIMED_OUT};
use executor::{IoContext, Wait};
use socket_base::{Endpoint, GetSocketOption, IoControl, Protocol, SetSocketOption, Socket};
use std::ffi::CStr;
use std::mem::{self, MaybeUninit};
use std::time::{Instant, Duration};

#[derive(Clone)]
pub struct Blocking {
    expire: i32,
}

impl Blocking {
    pub const fn infinit() -> Self {
        Blocking {
            expire: -1,
        }
    }

    pub fn expires_after(&mut self, expire: Duration) {
        self.expire = expire.as_millis() as i32
    }

    fn duration_from(&mut self, base: Instant) {
        self.expire -= (Instant::now() - base).as_millis() as i32
    }

    fn poll<P, S>(&mut self, soc: &S, event: i16) -> ErrorCode
        where S: Socket<P>,
    {
        let mut fds = libc::pollfd {
            fd: soc.native_handle(),
            events: event | libc::POLLERR | libc::POLLHUP,
            revents: 0,
        };
        loop {
            let now = Instant::now();
            let err = unsafe { libc::poll(&mut fds, 1, self.expire) };
            match err {
                1 => {
                    if (fds.revents & event) != 0 {
                        return SUCCESS;
                    } else {
                        self.duration_from(now);
                        return ErrorCode::socket_error(soc.native_handle());
                    }
                }
                0 => return TIMED_OUT,
                _ => match ErrorCode::last_error() {
                    INTERRUPTED => {}
                    err => {
                        self.duration_from(now);
                        return err;
                    }
                },
            }
        }

    }
}

impl Wait for Blocking {
    fn readable<P, S>(&mut self, soc: &S) -> ErrorCode
    where
        S: Socket<P>,
    {
        self.poll(soc, libc::POLLIN)
    }

    fn writable<P, S>(&mut self, soc: &S) -> ErrorCode
    where
        S: Socket<P>,
    {
        self.poll(soc, libc::POLLOUT)
    }
}

fn init(fd: NativeHandle) {
    unsafe {
        // set CLOEXEC flag
        libc::fcntl(
            fd,
            libc::F_SETFD,
            libc::FD_CLOEXEC | libc::fcntl(fd, libc::F_GETFD),
        );

        // set NONBLOCK flag
        libc::fcntl(
            fd,
            libc::F_SETFL,
            libc::O_NONBLOCK | libc::fcntl(fd, libc::F_GETFD),
        );
    }
}

pub fn close(fd: NativeHandle) -> Result<(), ErrorCode>
{
    let err = unsafe { libc::close(fd) };
    if err == 0 {
        Ok(())
    } else {
        Err(ErrorCode::last_error())
    }
}

pub fn ioctl<T>(fd: NativeHandle, data: &mut T) -> Result<(), ErrorCode>
    where T: IoControl
{
    let err = unsafe {
        libc::ioctl(fd, data.name(), data.as_mut_ptr())
    };
    if err >= 0 {
        Ok(())
    } else {
        Err(ErrorCode::last_error())
    }
}

#[allow(dead_code)]
pub fn pipe() -> Result<(NativeHandle, NativeHandle), ErrorCode> {
    let mut fds: [NativeHandle; 2] = unsafe { MaybeUninit::uninit().assume_init() };
    let err = unsafe { libc::pipe(fds.as_mut_ptr()) };
    if err == 0 {
        let [rfd, wfd] = fds;
        init(rfd);
        init(wfd);
        Ok((rfd, wfd))
    } else {
        Err(ErrorCode::last_error())
    }
}

pub fn socket<P, S>(ctx: &IoContext, pro: P) -> Result<S, ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
{
    let fd = unsafe { libc::socket(pro.family_type(), pro.socket_type(), pro.protocol_type()) };
    if fd >= 0 {
        init(fd);
        Ok(unsafe { S::unsafe_new(ctx, pro, fd) })
    } else {
        Err(ErrorCode::last_error())
    }
}

pub fn socketpair<P, S>(ctx: &IoContext, pro: &P) -> Result<(S, S), ErrorCode>
where
    P: Protocol + Clone,
    S: Socket<P>,
{
    let mut fds = unsafe { MaybeUninit::<[NativeHandle; 2]>::uninit().assume_init() };
    let err = unsafe {
        libc::socketpair(
            pro.family_type(),
            pro.socket_type(),
            pro.protocol_type(),
            fds.as_mut_ptr(),
        )
    };
    if err == 0 {
        let [fd1, fd2] = fds;
        init(fd1);
        init(fd2);
        Ok((unsafe { S::unsafe_new(ctx, pro.clone(), fd1) },
            unsafe { S::unsafe_new(ctx, pro.clone(), fd2) }))
    } else {
        Err(ErrorCode::last_error())
    }
}

pub fn accept<P, S>(soc: &S, pro: P, ctx: &IoContext) -> Result<(P::Socket, P::Endpoint), ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
{
    let mut ep = unsafe { pro.uninit().assume_init() };
    let mut len = ep.capacity();
    let fd = unsafe { libc::accept(soc.native_handle(), ep.as_mut_ptr(), &mut len) };
    if fd >= 0 {
        init(fd);
        unsafe { ep.resize(len); }
        Ok((unsafe { P::Socket::unsafe_new(ctx, pro, fd) }, ep))
    } else {
        Err(ErrorCode::last_error())
    }
}

pub fn bind<P, S>(soc: &S, ep: &P::Endpoint) -> Result<(), ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
{
    let err = unsafe { libc::bind(soc.native_handle(), ep.as_ptr(), ep.size()) };
    if err == 0 {
        Ok(())
    } else {
        Err(ErrorCode::last_error())
    }
}

pub fn listen<P, S>(soc: &S, backlog: i32) -> Result<(), ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
{
    let err = unsafe { libc::listen(soc.native_handle(), backlog) };
    if err == 0 {
        Ok(())
    } else {
        Err(ErrorCode::last_error())
    }
}

pub fn connect<P, S>(soc: &S, ep: &P::Endpoint) -> Result<(), ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
{
    let err = unsafe { libc::connect(soc.native_handle(), ep.as_ptr(), ep.size()) };
    if err == 0 {
        Ok(())
    } else {
        Err(ErrorCode::last_error())
    }
}

pub fn getpeername<P, S>(soc: &S, pro: &P) -> Result<P::Endpoint, ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
{
    let mut ep = unsafe { pro.uninit().assume_init() };
    let mut len = ep.capacity();
    let err = unsafe { libc::getpeername(soc.native_handle(), ep.as_mut_ptr(), &mut len) };
    if err == 0 {
        Ok(unsafe {
            ep.resize(len);
            ep
        })
    } else {
        Err(ErrorCode::last_error())
    }
}

pub fn getsockname<P, S>(soc: &S, pro: &P) -> Result<P::Endpoint, ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
{
    let mut ep = unsafe { pro.uninit().assume_init() };
    let mut len = ep.capacity();
    let err = unsafe { libc::getsockname(soc.native_handle(), ep.as_mut_ptr(), &mut len) };
    if err == 0 {
        Ok(unsafe {
            ep.resize(len);
            ep
        })
    } else {
        Err(ErrorCode::last_error())
    }
}

pub fn gethostname() -> Result<String, ErrorCode> {
    let mut name = unsafe { MaybeUninit::<[libc::c_char; 65]>::uninit().assume_init() };
    let err = unsafe { libc::gethostname(name.as_mut_ptr(), mem::size_of_val(&name)) };
    if err == 0 {
        let name = unsafe { CStr::from_ptr(name.as_ptr()) };
        Ok(String::from(name.to_str().unwrap()))
    } else {
        Err(ErrorCode::last_error())
    }
}

pub fn getsockopt<P, S, T>(soc: &S, pro: &P) -> Result<T, ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
    T: GetSocketOption<P>,
{
    let mut sockopt: T = unsafe { MaybeUninit::uninit().assume_init() };
    if let Some((level, name, ptr, mut len)) = sockopt.get_sockopt(pro) {
        let err = unsafe { libc::getsockopt(soc.native_handle(), level, name, ptr, &mut len) };
        if err == 0 {
            unsafe { sockopt.resize(len) };
            Ok(sockopt)
        } else {
            Err(ErrorCode::last_error())
        }
    } else {
        Err(INVALID_ARGUMENT)
    }
}

pub fn setsockopt<P, S, T>(soc: &S, pro: &P, sockopt: T) -> Result<(), ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
    T: SetSocketOption<P>,
{
    if let Some((level, name, ptr, len)) = sockopt.set_sockopt(pro) {
        let err = unsafe { libc::setsockopt(soc.native_handle(), level, name, ptr, len) };
        if err == 0 {
            Ok(())
        } else {
            Err(ErrorCode::last_error())
        }
    } else {
        Err(INVALID_ARGUMENT)
    }
}

pub fn shutdown<P, S>(soc: &S, how: i32) -> Result<(), ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
{
    let err = unsafe { libc::shutdown(soc.native_handle(), how.into()) };
    if err == 0 {
        Ok(())
    } else {
        Err(ErrorCode::last_error())
    }
}

pub fn read<P, S>(soc: &S, buf: &mut [u8]) -> Result<usize, ErrorCode>
where
    S: Socket<P>,
{
    let size = unsafe { libc::read(soc.native_handle(), buf.as_mut_ptr() as _, buf.len()) };
    if size > 0 {
        Ok(size as usize)
    } else if size == 0 {
        Err(CONNECTION_ABORTED)
    } else {
        Err(ErrorCode::last_error())
    }
}

pub fn write<P, S>(soc: &S, buf: &[u8]) -> Result<usize, ErrorCode>
where
    S: Socket<P>,
{
    let size = unsafe { libc::write(soc.native_handle(), buf.as_ptr() as _, buf.len()) };
    if size > 0 {
        Ok(size as usize)
    } else {
        Err(ErrorCode::last_error())
    }
}

pub fn send<P, S>(soc: &S, buf: &[u8], flags: i32) -> Result<usize, ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
{
    let size = unsafe { libc::send(soc.native_handle(), buf.as_ptr() as _, buf.len(), flags) };
    if size > 0 {
        Ok(size as usize)
    } else {
        Err(ErrorCode::last_error())
    }
}

pub fn sendto<P, S>(soc: &S, buf: &[u8], flags: i32, ep: &P::Endpoint) -> Result<usize, ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
{
    let size = unsafe {
        libc::sendto(
            soc.native_handle(),
            buf.as_ptr() as _,
            buf.len(),
            flags,
            ep.as_ptr(),
            ep.size(),
        )
    };
    if size > 0 {
        Ok(size as usize)
    } else {
        Err(ErrorCode::last_error())
    }
}

pub fn recv<P, S>(soc: &S, buf: &mut [u8], flags: i32) -> Result<usize, ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
{
    let size = unsafe {
        libc::recv(
            soc.native_handle(),
            buf.as_mut_ptr() as *mut _ as *mut _,
            buf.len(),
            flags,
        )
    };
    if size > 0 {
        Ok(size as usize)
    } else if size == 0 {
        Err(CONNECTION_ABORTED)
    } else {
        Err(ErrorCode::last_error())
    }
}

pub fn recvfrom<P, S>(
    soc: &S,
    buf: &mut [u8],
    flags: i32,
    pro: &P,
) -> Result<(usize, P::Endpoint), ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
{
    let mut ep = unsafe { pro.uninit().assume_init() };
    let mut len = ep.capacity();
    let size = unsafe {
        libc::recvfrom(
            soc.native_handle(),
            buf.as_mut_ptr() as *mut _ as *mut _,
            buf.len(),
            flags,
            ep.as_mut_ptr(),
            &mut len,
        )
    };
    if size > 0 {
        unsafe { ep.resize(len) };
        Ok((size as usize, ep))
    } else if size == 0 {
        Err(CONNECTION_ABORTED)
    } else {
        Err(ErrorCode::last_error())
    }
}
