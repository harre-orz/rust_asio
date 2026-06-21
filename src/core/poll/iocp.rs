use super::{Deadline, Intr, Scheduler};
use crate::error::OsError;
use crate::primitive::{AsRawHandle, Handle, Socket, Timeout};
use std::mem::MaybeUninit;
use std::pin::Pin;
use std::sync::{Mutex, MutexGuard};
use std::task::{Context, Poll, Waker};
use std::{mem, ptr};
use windows_sys::Win32::Foundation;
use windows_sys::Win32::Networking::WinSock;
use windows_sys::Win32::System::IO;

enum State {
    Result(Result<usize, OsError>),
    Queued(Waker),
}

struct Inner {
    state: State,
}

pub(crate) struct IocpEvent {
    inner: Mutex<Inner>,
    pub(crate) deadline: Deadline,
}

impl IocpEvent {
    pub(crate) fn new<T>(data: T) -> Pin<Box<(Self, T)>> {
        Box::pin((
            Self {
                inner: Mutex::new(Inner {
                    state: State::Result(Ok(0)),
                }),
                deadline: Deadline::UNINIT,
            },
            data,
        ))
    }
}

pub struct WaitForIocp<'a, 'b> {
    inner: &'a super::super::Inner,
    event: &'b IocpEvent,
    mutex: Option<MutexGuard<'b, Inner>>,
}

impl<'a, 'b> Future for WaitForIocp<'a, 'b> {
    type Output = Result<usize, OsError>;

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        if let Some(mut mutex) = self.mutex.take() {
            mutex.state = State::Queued(ctx.waker().clone());
            Poll::Pending
        } else {
            let event = self.event.lock().unwrap();
            match &event.state {
                State::Result(res) => Poll::Ready(*res),
                State::Queued(_) => Poll::Pending,
            }
        }
    }
}

unsafe impl<'a, 'b> Send for WaitForIocp<'a, 'b> {}

unsafe impl<'a, 'b> Sync for WaitForIocp<'a, 'b> {}

pub struct IocpEventGuard<'a, 'b> {
    inner: &'a super::super::Inner,
    mutex: MutexGuard<'b, Inner>,
    event: &'b IocpEvent,
}

impl<'a, 'b> IocpEventGuard<'a, 'b> {
    pub fn lock(inner: &'a super::super::Inner, event: &'b IocpEvent) -> IocpEventGuard<'a, 'b> {
        Self {
            inner: inner,
            event: event,
            mutex: event.inner.lock().unwrap(),
        }
    }

    pub fn poll_iocp(self, t: Timeout) -> WaitForIocp<'a, 'b> {
        WaitForIocp {
            inner: self.inner,
            event: self.event,
            mutex: Some(self.mutex),
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
    fn ex(&self) -> Result<WinSockEx, OsError> {
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
    pub(crate) fn new() -> Result<Self, OsError> {
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

fn iocp_new() -> Result<Handle, OsError> {
    const NULL_HANDLE: Foundation::HANDLE = ptr::null_mut();

    unsafe {
        match IO::CreateIoCompletionPort(Foundation::INVALID_HANDLE_VALUE, ptr::null_mut(), 0, 0) {
            NULL_HANDLE => Err(OsError::last()),
            handle => Ok(Handle::from_raw_handle(handle)),
        }
    }
}

fn iocp_add<T>(iocp: &Handle, handle: &T, ev: &IocpEvent)
where
    T: AsRawHandle,
{
    unsafe {
        IO::CreateIoCompletionPort(
            handle.as_raw_handle(),
            iocp.as_raw_handle(),
            ptr::from_ref(ev) as usize,
            0,
        );
    }
}

fn iocp_poll(
    iocp: &Handle,
    timeout: u32,
) -> Result<(Result<usize, OsError>, *const Mutex<Inner>), OsError> {
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
            Ok((Ok(len as usize), ev.assume_init().cast()))
        } else {
            let ev = ev.assume_init();
            if ev > 0 {
                Ok((Err(OsError::last()), ev.cast()))
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
    intr_event: Pin<Box<(IocpEvent, ())>>,
}

impl Iocp {
    pub(crate) fn new() -> Result<Iocp, OsError> {
        let ex = WinSockEx::new()?;
        let iocp = iocp_new()?;
        let intr = Intr::new()?;
        let intr_event = IocpEvent::new(());
        iocp_add(&iocp, &intr.as_handle(), &intr_event.0);
        Ok(Iocp {
            ex: ex,
            iocp: iocp,
            intr: intr,
            intr_event: intr_event,
        })
    }

    pub(crate) fn add_socket<T>(&self, soc: &Socket, event: Pin<&(IocpEvent, T)>) {
        iocp_add(&self.iocp, soc, &event.0)
    }

    pub(crate) fn del_socket<T>(&self, _soc: &Socket) {}

    pub fn cancel_all_events(&self, scheduler: &Scheduler) {}

    pub fn wake_up_now(&self) {}
    pub fn poll(&self, scheduler: &Scheduler) -> Poll<OsError> {
        match iocp_poll(&self.iocp, self.intr.timeout_iocp()) {
            Err(err) => Poll::Ready(err),
            Ok((res, mut event)) => {
                let event = unsafe { &*event };
                if ptr::addr_eq(event, &self.intr_event) {
                    self.intr.update_event();
                    return Poll::Pending;
                }
                let mut state = State::Result(res);
                let mut event = event.lock().unwrap();
                mem::swap(&mut state, &mut event.state);
                if let Some(waker) = state {
                    drop(event);
                    waker.wake()
                }
                Poll::Pending
            }
        }
    }
}
