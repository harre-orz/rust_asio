use crate::core::intr::Intr;
use crate::core::scheduler::Scheduler;
use crate::error::{OsError, Result};
use crate::primitive::{AsRawHandle, Handle, Socket};
use std::mem::MaybeUninit;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::{mem, ptr};
use windows_sys::Win32::Foundation;
use windows_sys::Win32::Networking::WinSock;
use windows_sys::Win32::System::IO;

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

struct SocketGuard(WinSock::SOCKET);

impl Drop for SocketGuard {
    fn drop(&mut self) {
        unsafe {
            WinSock::closesocket(self.0);
        }
    }
}

impl SocketGuard {
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
                        soc => Ok(SocketGuard(soc).ex()?),
                    }
                }
                err => Err(OsError::from_raw(err)),
            }
        }
    }
}

enum EventOp {
    Neutral,
    Pending(Waker),
    Ok(usize),
    Err(OsError),
}

pub(super) struct Inner {
    op: EventOp,
}

impl Inner {
    pub(super) fn poll(&mut self, ctx: &mut Context) -> Poll<Result<usize>> {
        match self.op {
            EventOp::Neutral => {
                self.op = EventOp::Pending(ctx.waker().clone());
                Poll::Pending
            }
            EventOp::Pending(_) => Poll::Pending,
            EventOp::Ok(len) => Poll::Ready(Ok(len)),
            EventOp::Err(err) => Poll::Ready(Err(err)),
        }
    }

    pub(super) fn cancel(&self, vec: &mut Vec<Waker>) {}
}

pub(crate) struct IocpEvent(Arc<(Socket, Mutex<Inner>)>);

impl IocpEvent {
    pub fn new(soc: Socket) -> Self {
        Self(Arc::new((
            soc,
            Mutex::new(Inner {
                op: EventOp::Neutral,
            }),
        )))
    }

    pub fn as_socket(&self) -> &Socket {
        &self.0.0
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

fn iocp_add<T>(iocp: &Handle, soc: &T, event: &IocpEvent)
where
    T: AsRawHandle,
{
    unsafe {
        IO::CreateIoCompletionPort(
            soc.as_raw_handle(),
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
            let event = IocpEvent(Arc::from_raw(ev.assume_init() as *mut Mutex<_>));
            Ok((Ok(len as usize), event))
        } else {
            let ev = ev.assume_init();
            if ev > 0 {
                let event = IocpEvent(Arc::from_raw(ev as *mut Mutex<_>));
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
        let intr = Intr::new()?;
        let intr_event: IocpEvent = Default::default();
        iocp_add(&iocp, intr.as_handle(), &intr_event);
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
                    self.intr.update_event()
                } else {
                    let waker = {
                        let mut op = match res {
                            Ok(len) => EventOp::Ok(len),
                            Err(err) => EventOp::Err(err),
                        };
                        let mut event = self.lock().unwrap();
                        mem::swap(&mut op, &mut event.op);
                        if let EventOp::Pending(waker) = op {
                            waker
                        } else {
                            return Poll::Pending;
                        }
                    };
                }
                Poll::Pending
            }
        }
    }
}
