use crate::core::intr::Intr;
use crate::core::scheduler::Scheduler;
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

#[derive(Copy, Clone)]
pub enum EventResult {
    Ready,
    Cancel,
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

enum EventOp {
    Result(Result<usize>),
    Wait(Waker),
}

struct Inner {
    op: EventOp,
}

impl Inner {
    fn result(&mut self, res: Result<usize>) -> Option<Waker> {
        let mut event = EventOp::Result(res);
        mem::swap(&mut self.op, &mut event);
        if let EventOp::Wait(waker) = event {
            Some(waker)
        } else {
            None
        }
    }
}

pub struct WaitForIocp<'a> {
    event: &'a IocpEvent,
    guard: Option<IocpEventGuard<'a>>,
}

impl<'a> Future for WaitForIocp<'a> {
    type Output = Result<usize>;

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        if let Some(guard) = self.guard.take() {
            guard.0.op = EventOp::Wait(ctx.waker().clone());
            Poll::Pending
        } else {
            let event = self.event.0.0.lock().unwrap();
            if let EventOp::Result(res) = event.op {
                Poll::Ready(res)
            } else {
                Poll::Pending
            }
        }
    }
}

unsafe impl<'a> Send for WaitForIocp<'a> {}

unsafe impl<'a> Sync for WaitForIocp<'a> {}

pub struct IocpEventGuard<'a>(MutexGuard<'a, Inner>);

impl<'a> IocpEventGuard<'a> {
    pub fn poll_iocp(self, event: &'a IocpEvent, t: Timeout) -> WaitForIocp<'a> {
        WaitForIocp {
            event: event,
            guard: Some(self),
        }
    }
}

union IocpData {
    handle: Handle,
    soc: Socket,
}

pub(crate) struct IocpEvent(Arc<(Mutex<Inner>, IocpData)>);

impl IocpEvent {
    pub fn new(soc: Socket) -> Self {
        Self(Arc::new((
            Mutex::new(Inner {
                op: EventOp::Result(Ok(0)),
            }),
            IocpData { soc: soc },
        )))
    }

    pub fn as_socket(&self) -> &Socket {
        unsafe { &self.0.1.soc }
    }

    pub fn lock(&self) -> IocpEventGuard<'_> {
        IocpEventGuard(self.0.0.lock().unwrap())
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

fn iocp_add<T>(iocp: &Handle, event: &IocpEvent)
where
    T: AsRawHandle,
{
    unsafe {
        IO::CreateIoCompletionPort(
            event.as_socket().as_raw_handle(),
            iocp.as_raw_handle(),
            Arc::into_raw(event.0.clone()) as usize,
            0,
        );
    }
}

fn iocp_poll(iocp: &Handle, timeout: u32) -> Result<(Result<usize>, IocpEvent)> {
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
    intr_event: IocpEvent,
}

impl Iocp {
    pub(super) fn new() -> Result<Iocp> {
        let ex = WinSockEx::new()?;
        let iocp = iocp_new()?;
        let (intr, handle) = Intr::new()?;
        let intr_event = IocpEvent(Arc::new((
            Mutex::new(Inner {
                op: EventOp::Result(Ok(0)),
            }),
            IocpData { handle: handle },
        )));
        iocp_add(&iocp, &intr_event);
        Ok(Iocp {
            ex: ex,
            iocp: iocp,
            intr: intr,
            intr_event: intr_event,
        })
    }

    pub(crate) fn add_socket(&self, ev: &IocpEvent) {
        iocp_add(&self.iocp, ev)
    }

    pub(crate) fn del_socket(&self, ev: &IocpEvent) {}

    pub(super) fn poll(&self, scheduler: &Scheduler) -> Poll<OsError> {
        match iocp_poll(&self.iocp, self.intr.timeout().as_millis() as u32) {
            Err(err) => Poll::Ready(err),
            Ok((res, mut event)) => {
                if ptr::eq(&event, &self.intr_event) {
                    self.intr
                        .update_event(unsafe { &self.intr_event.0.1.handle });
                    return Poll::Pending;
                }
                let waker = {
                    let mut event = event.0.0.lock().unwrap();
                    if let Some(waker) = event.result(res) {
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
