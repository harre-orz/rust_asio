use super::AsyncSocket;
use crate::buffer::MsgBuf;
use crate::core::IoContext;
use crate::error::OsError;
use crate::primitive::{Fd, Socket};
use crate::sockaddr::{SockAddr, SockLen};
use crate::socket_base::{Endpoint, EndpointRef, GetSockOpt, Protocol, SetSockOpt, Shutdown};
use std::mem::MaybeUninit;
use std::{mem, ptr};

impl Socket {
    pub(crate) fn new<P>(pro: P) -> Result<Self, OsError>
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
                fd => {
                    let fd = Fd::from_raw_fd(fd);
                    #[cfg(target_os = "macos")]
                    fd.set_cloexec()?;
                    #[cfg(target_os = "macos")]
                    fd.set_nonblock()?;
                    Ok(Socket(fd))
                }
            }
        }
    }

    pub(crate) fn pair<P>(pro: P) -> Result<(Self, Self), OsError>
    where
        P: Protocol,
    {
        let mut fds: [MaybeUninit<libc::c_int>; 2] = [const { MaybeUninit::uninit() }; 2];
        let socktype: i32 = pro.socket_type().into();
        #[cfg(target_os = "linux")]
        let socktype = socktype | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK;
        unsafe {
            match libc::socketpair(
                pro.family_type().into(),
                socktype,
                pro.protocol_type().into(),
                fds[0].as_mut_ptr(),
            ) {
                -1 => Err(OsError::last()),
                _ => {
                    let fds = mem::transmute::<_, [libc::c_int; 2]>(fds);
                    let fd1 = Fd::from_raw_fd(fds[0]);
                    let fd2 = Fd::from_raw_fd(fds[1]);
                    #[cfg(target_os = "macos")]
                    fd1.set_cloexec()?;
                    #[cfg(target_os = "macos")]
                    fd2.set_cloexec()?;
                    #[cfg(target_os = "macos")]
                    fd1.set_nonblock()?;
                    #[cfg(target_os = "macos")]
                    fd2.set_nonblock()?;
                    Ok((Socket(fd1), Socket(fd2)))
                }
            }
        }
    }

    pub(crate) fn close(self) -> Result<(), OsError> {
        self.0.close()
    }

    pub(crate) fn bind<E>(&self, ep: &EndpointRef<E>) -> Result<(), OsError>
    where
        E: Endpoint,
    {
        unsafe {
            let sa = ptr::from_ref(ep.sockaddr_ref()).cast();
            match libc::bind(self.0.as_raw_fd(), sa, ep.sockaddr_len()) {
                -1 => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub(crate) fn listen(&self, backlog: i32) -> Result<(), OsError> {
        unsafe {
            match libc::listen(self.0.as_raw_fd(), backlog) {
                -1 => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub(crate) fn getsockname<E>(&self) -> Result<E, OsError>
    where
        E: Endpoint,
    {
        let mut sa = MaybeUninit::<E::SockAddr>::uninit();
        let mut sa_len = size_of::<E::SockAddr>() as SockLen;
        unsafe {
            match libc::getsockname(self.0.as_raw_fd(), sa.as_mut_ptr().cast(), &mut sa_len) {
                -1 => Err(OsError::last()),
                _ => Ok(E::from_sockaddr(E::SockAddr::init(sa, sa_len))),
            }
        }
    }

    pub(crate) fn getpeername<E>(&self) -> Result<E, OsError>
    where
        E: Endpoint,
    {
        let mut sa = MaybeUninit::<E::SockAddr>::uninit();
        let mut sa_len = size_of::<E::SockAddr>() as SockLen;
        unsafe {
            match libc::getpeername(self.0.as_raw_fd(), sa.as_mut_ptr().cast(), &mut sa_len) {
                -1 => Err(OsError::last()),
                _ => Ok(E::from_sockaddr(E::SockAddr::init(sa, sa_len))),
            }
        }
    }

    pub(crate) fn shutdown(&self, how: Shutdown) -> Result<(), OsError> {
        unsafe {
            match libc::shutdown(self.0.as_raw_fd(), how as i32) {
                -1 => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub(crate) fn setsockopt<P>(&self, pro: P, opt: &dyn SetSockOpt<P>) -> Result<(), OsError>
    where
        P: Protocol,
    {
        let (key, data) = opt.data(pro);
        unsafe {
            match libc::setsockopt(
                self.0.as_raw_fd(),
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

    pub(crate) fn getsockopt<P, S>(&self, pro: P) -> Result<S, OsError>
    where
        P: Protocol,
        S: GetSockOpt<P>,
    {
        let (key, init) = S::init(pro);
        let mut data = MaybeUninit::<S>::uninit();
        let mut data_len = size_of::<S>() as SockLen;
        unsafe {
            match libc::getsockopt(
                self.0.as_raw_fd(),
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

    pub(crate) fn nb_accept<E>(&self) -> Result<(Socket, E), OsError>
    where
        E: Endpoint,
    {
        let mut sa = MaybeUninit::<E::SockAddr>::uninit();
        let mut sa_len = size_of::<E::SockAddr>() as SockLen;
        unsafe {
            #[cfg(target_os = "linux")]
            let res = libc::accept4(
                self.0.as_raw_fd(),
                sa.as_mut_ptr().cast(),
                &mut sa_len,
                libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
            );
            #[cfg(target_os = "macos")]
            let res = libc::accept(self.0.as_raw_fd(), sa.as_mut_ptr().cast(), &mut sa_len);
            match res {
                -1 => Err(OsError::last()),
                fd => {
                    let fd = Fd::from_raw_fd(fd);
                    #[cfg(target_os = "macos")]
                    fd.set_cloexec()?;
                    #[cfg(target_os = "macos")]
                    fd.set_nonblock()?;
                    let ep = E::from_sockaddr(E::SockAddr::init(sa, sa_len));
                    Ok((Socket(fd), ep))
                }
            }
        }
    }

    pub(crate) fn nb_connect<E>(&self, ep: &EndpointRef<E>) -> Result<(), OsError>
    where
        E: Endpoint,
    {
        unsafe {
            let sa = ptr::from_ref(ep.sockaddr_ref()).cast();
            match libc::connect(self.0.as_raw_fd(), sa, ep.sockaddr_len()) {
                -1 => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub(crate) fn nb_recv(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        unsafe {
            match libc::recv(self.0.as_raw_fd(), buf.as_mut_ptr().cast(), buf.len(), 0) {
                -1 => Err(OsError::last()),
                0 => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }

    pub(crate) fn nb_recvfrom<E>(&self, buf: &mut [u8]) -> Result<(usize, E), OsError>
    where
        E: Endpoint,
    {
        let mut sa = MaybeUninit::<E::SockAddr>::uninit();
        let mut sa_len = size_of::<E::SockAddr>() as SockLen;
        unsafe {
            match libc::recvfrom(
                self.0.as_raw_fd(),
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
    pub(crate) fn nb_recvmsg(&self, msg: &mut MsgBuf, _: &IoContext) -> Result<usize, OsError> {
        unsafe {
            match libc::recvmsg(self.0.as_raw_fd(), msg.as_msghdr(), 0) {
                -1 => Err(OsError::last()),
                0 => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }

    #[cfg(target_os = "linux")]
    pub(crate) fn nb_recvmsg(&self, msg: &mut MsgBuf, _: &IoContext) -> Result<usize, OsError> {
        match msg.mmsghdr_recv_next() {
            Ok(len) => Ok(len),
            Err(mmsghdr) =>
                unsafe {
                    match libc::recvmmsg(
                        self.0.as_raw_fd(),
                        mmsghdr.as_mut_ptr(),
                        mmsghdr.len() as SockLen,
                        0,
                        ptr::null_mut(),
                    ) {
                        -1 => Err(OsError::last()),
                        0 => Err(OsError::CONNECTION_ABORTED),
                        len => Ok(msg.mmsghdr_set_len(len as usize)),
                    }
                },
        }
    }

    pub(crate) fn nb_send(&self, buf: &[u8]) -> Result<usize, OsError> {
        unsafe {
            match libc::send(self.0.as_raw_fd(), buf.as_ptr().cast(), buf.len(), 0) {
                -1 => Err(OsError::last()),
                0 if !buf.is_empty() => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }

    pub(crate) fn nb_sendto<E>(&self, buf: &[u8], ep: &EndpointRef<E>) -> Result<usize, OsError>
    where
        E: Endpoint,
    {
        unsafe {
            let sa = ptr::from_ref(ep.sockaddr_ref()).cast();
            match libc::sendto(
                self.0.as_raw_fd(),
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
    pub(crate) fn nb_sendmsg(&self, msg: &mut MsgBuf) -> Result<usize, OsError> {
        unsafe {
            match libc::sendmsg(self.0.as_raw_fd(), msg.as_msghdr(), 0) {
                -1 => Err(OsError::last()),
                0 => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }

    #[cfg(target_os = "linux")]
    pub(crate) fn nb_sendmsg(&self, msg: &mut MsgBuf) -> Result<usize, OsError> {
        Err(OsError::CONNECTION_ABORTED)
        // unsafe {
        //     match libc::sendmmsg(
        //         self.0.as_raw_fd(),
        //         mmsghdr.as_mut_ptr(),
        //         mmsghdr.len() as SockLen,
        //         0,
        //     ) {
        //         -1 => Err(OsError::last()),
        //         0 => Err(OsError::CONNECTION_ABORTED),
        //         len => Ok(msg.mmsghdr_set_len(len as usize)),
        //     }
        // }
    }

    pub(crate) fn nb_read(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.0.read(buf)
    }

    pub(crate) fn nb_write(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.0.write(buf)
    }
}

impl<T> AsyncSocket<T> {
    pub(crate) async fn async_accept<E>(&self) -> Result<(Socket, E), OsError>
    where
        E: Endpoint,
    {
        let (ev, (ctx, soc, ato, _)) = &*self.0;
        if ctx.is_stopped() {
            return Err(OsError::OPERATION_CANCELED);
        }
        loop {
            let guard = ctx.lock(ev);
            match soc.nb_accept() {
                Ok(soc) => return Ok(soc),
                #[allow(unreachable_patterns)]
                Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => {
                    match guard.poll_in(ato.get()).await {
                        Ok(()) => {}
                        Err(()) => return Err(OsError::OPERATION_CANCELED),
                    }
                }
                Err(OsError::INTERRUPTED) => {}
                Err(err) => return Err(err),
            }
        }
    }

    pub(crate) async fn async_connect<E>(&self, ep: &EndpointRef<'_, E>) -> Result<(), OsError>
    where
        E: Endpoint,
    {
        let (ev, (ctx, soc, ato, _)) = &*self.0;
        if ctx.is_stopped() {
            return Err(OsError::OPERATION_CANCELED);
        }
        loop {
            let guard = ctx.lock(ev);
            match soc.nb_connect(ep) {
                Ok(_) => return Ok(()),
                Err(OsError::IN_PROGRESS) | Err(OsError::WOULD_BLOCK) => {
                    match guard.poll_out(ato.get()).await {
                        Ok(()) => return Ok(()),
                        Err(()) => return Err(OsError::OPERATION_CANCELED),
                    }
                }
                Err(OsError::INTERRUPTED) => {}
                Err(err) => return Err(err),
            }
        }
    }

    pub(crate) async fn async_write(&self, buf: &[u8]) -> Result<usize, OsError> {
        let (ev, (ctx, soc, ato, _)) = &*self.0;
        if ctx.is_stopped() {
            return Err(OsError::OPERATION_CANCELED);
        }
        loop {
            let guard = ctx.lock(ev);
            match soc.nb_write(buf) {
                Ok(len) => return Ok(len),
                #[allow(unreachable_patterns)]
                Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => {
                    match guard.poll_out(ato.get()).await {
                        Ok(()) => {}
                        Err(()) => return Err(OsError::OPERATION_CANCELED),
                    }
                }
                Err(OsError::INTERRUPTED) => {}
                Err(err) => return Err(err),
            }
        }
    }

    pub(crate) async fn async_send(&self, buf: &[u8]) -> Result<usize, OsError> {
        let (ev, (ctx, soc, ato, _)) = &*self.0;
        if ctx.is_stopped() {
            return Err(OsError::OPERATION_CANCELED);
        }
        loop {
            let guard = ctx.lock(ev);
            match soc.nb_send(buf) {
                Ok(len) => return Ok(len),
                #[allow(unreachable_patterns)]
                Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => {
                    match guard.poll_out(ato.get()).await {
                        Ok(()) => {}
                        Err(()) => return Err(OsError::OPERATION_CANCELED),
                    }
                }
                Err(OsError::INTERRUPTED) => {}
                Err(err) => return Err(err),
            }
        }
    }

    pub(crate) async fn async_sendto<E>(
        &self,
        buf: &[u8],
        ep: &EndpointRef<'_, E>,
    ) -> Result<usize, OsError>
    where
        E: Endpoint,
    {
        let (ev, (ctx, soc, ato, _)) = &*self.0;
        if ctx.is_stopped() {
            return Err(OsError::OPERATION_CANCELED);
        }
        loop {
            let guard = ctx.lock(ev);
            match soc.nb_sendto(buf, ep) {
                Ok(len) => return Ok(len),
                #[allow(unreachable_patterns)]
                Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => {
                    match guard.poll_out(ato.get()).await {
                        Ok(()) => {}
                        Err(()) => return Err(OsError::OPERATION_CANCELED),
                    }
                }
                Err(OsError::INTERRUPTED) => {}
                Err(err) => return Err(err),
            }
        }
    }

    pub(crate) async fn async_sendmsg(&self, msg: &mut MsgBuf) -> Result<usize, OsError> {
        let (ev, (ctx, soc, ato, _)) = &*self.0;
        if ctx.is_stopped() {
            return Err(OsError::OPERATION_CANCELED);
        }
        loop {
            let guard = ctx.lock(ev);
            match soc.nb_sendmsg(msg) {
                Ok(len) => return Ok(len),
                #[allow(unreachable_patterns)]
                Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => {
                    match guard.poll_out(ato.get()).await {
                        Ok(()) => {}
                        Err(()) => return Err(OsError::OPERATION_CANCELED),
                    }
                }
                Err(OsError::INTERRUPTED) => {}
                Err(err) => return Err(err),
            }
        }
    }

    pub(crate) async fn async_read(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        let (ev, (ctx, soc, ato, _)) = &*self.0;
        if ctx.is_stopped() {
            return Err(OsError::OPERATION_CANCELED);
        }
        loop {
            let guard = ctx.lock(ev);
            match soc.nb_read(buf) {
                Ok(len) => return Ok(len),
                #[allow(unreachable_patterns)]
                Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => {
                    match guard.poll_in(ato.get()).await {
                        Ok(()) => {}
                        Err(()) => return Err(OsError::OPERATION_CANCELED),
                    }
                }
                Err(OsError::INTERRUPTED) => {}
                Err(err) => return Err(err),
            }
        }
    }

    pub(crate) async fn async_recv(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        let (ev, (ctx, soc, ato, _)) = &*self.0;
        if ctx.is_stopped() {
            return Err(OsError::OPERATION_CANCELED);
        }
        loop {
            let guard = ctx.lock(ev);
            match soc.nb_recv(buf) {
                Ok(len) => return Ok(len),
                #[allow(unreachable_patterns)]
                Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => {
                    match guard.poll_in(ato.get()).await {
                        Ok(()) => {}
                        Err(()) => return Err(OsError::OPERATION_CANCELED),
                    }
                }
                Err(OsError::INTERRUPTED) => {}
                Err(err) => return Err(err),
            }
        }
    }

    pub(crate) async fn async_recvfrom<E>(&self, buf: &mut [u8]) -> Result<(usize, E), OsError>
    where
        E: Endpoint,
    {
        let (ev, (ctx, soc, ato, _)) = &*self.0;
        if ctx.is_stopped() {
            return Err(OsError::OPERATION_CANCELED);
        }
        loop {
            let guard = ctx.lock(ev);
            match soc.nb_recvfrom(buf) {
                Ok(len) => return Ok(len),
                #[allow(unreachable_patterns)]
                Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => {
                    match guard.poll_in(ato.get()).await {
                        Ok(()) => {}
                        Err(()) => return Err(OsError::OPERATION_CANCELED),
                    }
                }
                Err(OsError::INTERRUPTED) => {}
                Err(err) => return Err(err),
            }
        }
    }

    pub(crate) async fn async_recvmsg(&self, msg: &mut MsgBuf) -> Result<usize, OsError> {
        let (ev, (ctx, soc, ato, _)) = &*self.0;
        if ctx.is_stopped() {
            return Err(OsError::OPERATION_CANCELED);
        }
        loop {
            let guard = ctx.lock(ev);
            match soc.nb_recvmsg(msg, ctx) {
                Ok(len) => return Ok(len),
                #[allow(unreachable_patterns)]
                Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => {
                    match guard.poll_in(ato.get()).await {
                        Ok(()) => {}
                        Err(()) => return Err(OsError::OPERATION_CANCELED),
                    }
                }
                Err(OsError::INTERRUPTED) => {}
                Err(err) => return Err(err),
            }
        }
    }
}
