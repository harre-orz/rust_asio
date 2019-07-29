//

pub type NativeHandle = std::os::unix::io::RawFd;

use error::{ErrorCode, CONNECTION_ABORTED, INTERRUPTED, INVALID_ARGUMENT, SUCCESS};
use executor::{IoContext, Blocking};
use socket_base::{Endpoint, GetSocketOption, IoControl, Protocol, SetSocketOption, Socket};
use std::ffi::CStr;
use std::mem;
use std::time::{Instant, Duration};

pub struct Expire(Instant);

impl Expire {
    pub fn new(dur: Duration) -> Expire {
        Expire(Instant::now() + dur)
    }

    fn duration_from_now(&self) -> i32 {
        // FIXME
        let dur = self.0.duration_since(Instant::now());
        if dur < Duration::new(0, 0) {
            0
        } else if dur > Duration::new(100, 0) {
            0
        } else {
            0
        }
    }
}

impl Blocking for Expire {
    fn ready_reading<P, S>(&mut self, soc: &S) -> ErrorCode
    where
        P: Protocol,
        S: Socket<P>,
    {
        let mut fds = libc::pollfd {
            fd: soc.native_handle(),
            events: libc::POLLIN | libc::POLLERR | libc::POLLHUP,
            revents: 0,
        };
        loop {
            let err = unsafe { libc::poll(&mut fds, 1, self.duration_from_now()) };
            if err >= 0 {
                if (fds.revents & libc::POLLIN) != 0 {
                    return SUCCESS;
                } else {
                    return ErrorCode::socket_error(soc.native_handle());
                }
            } else {
                match ErrorCode::last_error() {
                    INTERRUPTED => {}
                    err => return err,
                }
            }
        }
    }

    fn ready_writing<P, S>(&mut self, soc: &S) -> ErrorCode
    where
        P: Protocol,
        S: Socket<P>,
    {
        let mut fds = libc::pollfd {
            fd: soc.native_handle(),
            events: libc::POLLOUT | libc::POLLERR | libc::POLLHUP,
            revents: 0,
        };
        loop {
            let err = unsafe { libc::poll(&mut fds, 1, self.duration_from_now()) };
            if err >= 0 {
                if (fds.revents & libc::POLLOUT) != 0 {
                    return SUCCESS;
                } else {
                    return ErrorCode::socket_error(soc.native_handle());
                }
            } else {
                match ErrorCode::last_error() {
                    INTERRUPTED => {}
                    err => return err,
                }
            }
        }
    }
}

unsafe fn init(soc: NativeHandle) {
    // set CLOEXEC flag
    libc::fcntl(
        soc,
        libc::F_SETFD,
        libc::FD_CLOEXEC | libc::fcntl(soc, libc::F_GETFD),
    );

    // set NONBLOCK flag
    libc::fcntl(
        soc,
        libc::F_GETFL,
        libc::O_NONBLOCK | libc::fcntl(soc, libc::F_GETFD),
    );
}

pub fn accept<P, S>(soc: &S, pro: P) -> Result<(P::Socket, P::Endpoint), ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
{
    let ctx = soc.as_ctx();
    let mut ep = unsafe { pro.uninitialized() };
    let mut len = ep.capacity();
    let soc = unsafe { libc::accept(soc.native_handle(), ep.as_mut_ptr(), &mut len) };
    if soc >= 0 {
        Ok(unsafe {
            init(soc);
            ep.resize(len);
            (P::Socket::unsafe_new(ctx, pro, soc), ep)
        })
    } else {
        Err(ErrorCode::last_error())
    }
}

pub fn socket<P, S>(ctx: &IoContext, pro: P) -> Result<S, ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
{
    let soc = unsafe { libc::socket(pro.family_type(), pro.socket_type(), pro.protocol_type()) };
    if soc >= 0 {
        Ok(unsafe {
            init(soc);
            S::unsafe_new(ctx, pro, soc)
        })
    } else {
        Err(ErrorCode::last_error())
    }
}

pub fn socketpair<P, S>(ctx: &IoContext, pro1: P, pro2: P) -> Result<(S, S), ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
{
    let mut fds: [NativeHandle; 2] = unsafe { mem::uninitialized() };
    let err = unsafe {
        libc::socketpair(
            pro1.family_type(),
            pro1.socket_type(),
            pro1.protocol_type(),
            fds.as_mut_ptr(),
        )
    };
    if err == 0 {
        Ok(unsafe {
            let [soc1, soc2] = fds;
            init(soc1);
            init(soc2);
            (
                S::unsafe_new(ctx, pro1, soc1),
                S::unsafe_new(ctx, pro2, soc2),
            )
        })
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
    let mut ep = unsafe { pro.uninitialized() };
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
    let mut ep = unsafe { pro.uninitialized() };
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
    let mut name: [libc::c_char; 65] = unsafe { mem::uninitialized() };
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
    let mut sockopt: T = unsafe { mem::uninitialized() };
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

pub fn ioctl<P, S, T>(soc: &S, ctl: &mut T) -> Result<(), ErrorCode>
where
    S: Socket<P>,
    T: IoControl,
{
    let err = unsafe { libc::ioctl(soc.native_handle(), ctl.name(), ctl.as_mut_ptr()) };
    if err >= 0 {
        Ok(())
    } else {
        Err(ErrorCode::last_error())
    }
}

pub fn close<P, S>(soc: &S) -> Result<(), ErrorCode>
where
    S: Socket<P>,
{
    let err = unsafe { libc::close(soc.native_handle()) };
    if err == 0 {
        Ok(())
    } else {
        Err(ErrorCode::last_error())
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
    let mut ep = unsafe { pro.uninitialized() };
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

#[allow(dead_code)]
pub fn pipe() -> Result<(NativeHandle, NativeHandle), ErrorCode> {
    let mut fds: [NativeHandle; 2] = unsafe { mem::uninitialized() };
    let err = unsafe { libc::pipe(fds.as_mut_ptr()) };
    if err == 0 {
        let [rfd, wfd] = fds;
        unsafe {
            init(rfd);
            init(wfd);
        }
        Ok((rfd, wfd))
    } else {
        Err(ErrorCode::last_error())
    }
}
