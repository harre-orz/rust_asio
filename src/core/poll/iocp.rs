use super::Intr;
use crate::core::x::Scheduler;
use crate::error::{OsError, Result};
use crate::primitive::{AsRawHandle, Handle, Socket, Timeout};
use std::mem::MaybeUninit;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Context, Poll, Waker};
use std::{mem, ptr};
use windows_sys::Win32::Foundation;
use windows_sys::Win32::Networking::WinSock;
use windows_sys::Win32::System::IO;

enum State {
    Result(Result<usize>),
    Queued(Waker),
}

impl State {
    fn result(&mut self, res: Result<usize>) -> Option<Waker> {
        let mut state = State::Result(res);
        mem::swap(self, &mut state);
        if let Self::Queued(waker) = state {
            Some(waker)
        } else {
            None
        }
    }
}

struct Inner {
    state: State,
}

impl Inner {
    fn new() -> Self {
        Self {
            state: State::Result(Ok(0)),
        }
    }
}

pub struct WaitForIocp<'a, T> {
    event: &'a IocpEvent<T>,
    guard: Option<IocpEventGuard<'a, T>>,
}

impl<'a, T> Future for WaitForIocp<'a, T> {
    type Output = Result<usize>;

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        if let Some(mut guard) = self.guard.take() {
            guard.mutex.state = State::Queued(ctx.waker().clone());
            Poll::Pending
        } else {
            let event = self.event.0.0.lock().unwrap();
            match event.state {
                State::Result(res) => Poll::Ready(res),
                _ => Poll::Pending,
            }
        }
    }
}

unsafe impl<'a, T> Send for WaitForIocp<'a, T> {}

unsafe impl<'a, T> Sync for WaitForIocp<'a, T> {}

pub struct IocpEventGuard<'a, T> {
    mutex: MutexGuard<'a, Inner>,
    event: &'a IocpEvent<T>,
}

impl<'a, T> IocpEventGuard<'a, T> {
    pub fn poll_iocp(self, t: Timeout) -> WaitForIocp<'a, T> {
        WaitForIocp {
            event: self.event,
            guard: Some(self),
        }
    }
}

pub(crate) struct IocpEvent<T>(Arc<(Mutex<Inner>, T)>);

impl<T> IocpEvent<T> {
    pub fn new(data: T) -> Self {
        Self(Arc::new((Mutex::new(Inner::new()), data)))
    }

    pub fn as_data(&self) -> &T {
        &self.0.1
    }

    pub fn lock(&self) -> IocpEventGuard<'_, T> {
        IocpEventGuard {
            mutex: self.0.0.lock().unwrap(),
            event: self,
        }
    }
}

pub(crate) struct WinSockEx {
    pub ConnectEx: unsafe fn(
        s: WinSock::SOCKET,
        name: *const WinSock::SOCKADDR,
        namelen: i32,
        lpsendbuffer: *const core::ffi::c_void,
        dwsenddatalength: u32,
        lpdwbytessent: *mut u32,
        lpoverlapped: *mut IO::OVERLAPPED,
    ) -> windows_sys::core::BOOL,
    pub WSARecvMsg: unsafe fn(
        s: WinSock::SOCKET,
        lpmsg: *mut WinSock::WSAMSG,
        lpdwnumberofbytesrecvd: *mut u32,
        lpoverlapped: *mut IO::OVERLAPPED,
        lpcompletionroutine: WinSock::LPWSAOVERLAPPED_COMPLETION_ROUTINE,
    ) -> i32,
}

impl Drop for WinSockEx {
    fn drop(&mut self) {
        unsafe {
            let _ = WinSock::WSACleanup();
        }
    }
}

struct WinSockGuard(WinSock::SOCKET);

impl Drop for WinSockGuard {
    fn drop(&mut self) {
        unsafe {
            WinSock::closesocket(self.0);
        }
    }
}

impl WinSockGuard {
    fn ex(&self) -> Result<WinSockEx> {
        unsafe {
            let ConnectEx = {
                let guid = WinSock::WSAID_CONNECTEX;
                let mut lpfn: WinSock::LPFN_CONNECTEX = None;
                let mut bytes = 0;
                match WinSock::WSAIoctl(
                    self.0,
                    WinSock::SIO_GET_EXTENSION_FUNCTION_POINTER,
                    ptr::from_ref(&guid).cast(),
                    size_of_val(&guid) as _,
                    ptr::from_mut(&mut lpfn).cast(),
                    size_of_val(&lpfn) as _,
                    &mut bytes,
                    ptr::null_mut(),
                    None,
                ) {
                    WinSock::SOCKET_ERROR => return Err(OsError::last()),
                    _ => lpfn.unwrap(),
                }
            };

            let WSARecvMsg = {
                let guid = WinSock::WSAID_WSARECVMSG;
                let mut lpfn: WinSock::LPFN_WSARECVMSG = None;
                let mut bytes = 0;
                match WinSock::WSAIoctl(
                    self.0,
                    WinSock::SIO_GET_EXTENSION_FUNCTION_POINTER,
                    ptr::from_ref(&guid).cast(),
                    size_of_val(&guid) as _,
                    ptr::from_mut(&mut lpfn).cast(),
                    size_of_val(&lpfn) as _,
                    &mut bytes,
                    ptr::null_mut(),
                    None,
                ) {
                    WinSock::SOCKET_ERROR => return Err(OsError::last()),
                    _ => lpfn.unwrap(),
                }
            };
            Ok(WinSockEx {
                ConnectEx: ConnectEx,
                WSARecvMsg: WSARecvMsg,
            })
        }
    }
}

impl WinSockEx {
    pub(crate) fn new() -> Result<Self> {
        unsafe {
            let mut _data = MaybeUninit::<WinSock::WSADATA>::uninit();
            match WinSock::WSAStartup(0x0202, _data.as_mut_ptr()) {
                0 => {
                    match WinSock::WSASocketW(
                        WinSock::AF_INET as i32,
                        WinSock::SOCK_STREAM,
                        WinSock::IPPROTO_IP,
                        ptr::null_mut(),
                        0,
                        0,
                    ) {
                        WinSock::INVALID_SOCKET => Err(OsError::last()),
                        soc => Ok(WinSockGuard(soc).ex()?),
                    }
                }
                err => Err(OsError::from_raw(err)),
            }
        }
    }
}

fn iocp_new() -> Result<Handle> {
    const NULL_HANDLE: Foundation::HANDLE = ptr::null_mut();

    unsafe {
        match IO::CreateIoCompletionPort(Foundation::INVALID_HANDLE_VALUE, ptr::null_mut(), 0, 0) {
            NULL_HANDLE => Err(OsError::last()),
            handle => Ok(Handle::from_raw_handle(handle)),
        }
    }
}

fn iocp_add<T, U>(iocp: &Handle, handle: &T, event: &IocpEvent<U>)
where
    T: AsRawHandle,
{
    unsafe {
        IO::CreateIoCompletionPort(
            handle.as_raw_handle(),
            iocp.as_raw_handle(),
            Arc::into_raw(event.0.clone()) as usize,
            0,
        );
    }
}

fn iocp_poll(iocp: &Handle, timeout: u32) -> Result<(Result<usize>, IocpEvent<()>)> {
    let mut len = MaybeUninit::uninit();
    let mut ev = MaybeUninit::uninit();
    let mut ov = ptr::null_mut();
    unsafe {
        if IO::GetQueuedCompletionStatus(
            iocp.as_raw_handle(),
            len.as_mut_ptr(),
            ev.as_mut_ptr(),
            &mut ov,
            timeout,
        ) > 0
        {
            let len = unsafe { len.assume_init() };
            let event = IocpEvent(Arc::from_raw(ev.assume_init().cast()));
            Ok((Ok(len as usize), event))
        } else {
            let ev = ev.assume_init();
            if ev > 0 {
                let event = IocpEvent(Arc::from_raw(ev.cast()));
                Ok((Err(OsError::last()), event))
            } else {
                Err(OsError::last())
            }
        }
    }
}

pub(in super::super) struct Iocp {
    ex: WinSockEx,
    iocp: Handle,
    intr: Intr,
    intr_event: IocpEvent<()>,
}

impl Iocp {
    pub(super) fn new() -> Result<Iocp> {
        let ex = WinSockEx::new()?;
        let iocp = iocp_new()?;
        let intr = Intr::new()?;
        let intr_event = IocpEvent::new(());
        iocp_add(&iocp, &intr.as_handle(), &intr_event);
        Ok(Iocp {
            ex: ex,
            iocp: iocp,
            intr: intr,
            intr_event: intr_event,
        })
    }

    pub(crate) fn add_socket<T>(&self, soc: &Socket, ev: &IocpEvent<T>) {
        iocp_add(&self.iocp, soc, ev)
    }

    pub(crate) fn del_socket<T>(&self, _soc: &Socket) {}

    pub(super) fn poll(&self, scheduler: &Scheduler) -> Poll<OsError> {
        match iocp_poll(&self.iocp, self.intr.timeout().as_millis() as u32) {
            Err(err) => Poll::Ready(err),
            Ok((res, mut event)) => {
                if ptr::eq(&event, &self.intr_event) {
                    self.intr.update_event();
                    return Poll::Pending;
                }
                let waker = {
                    let mut event = event.0.0.lock().unwrap();
                    if let Some(waker) = event.state.result(res) {
                        waker
                    } else {
                        return Poll::Pending;
                    }
                };
                waker.wake();
                Poll::Pending
            }
        }
    }
}
