use super::{AsRawHandle, EventScheduler, Handle, Intr};
use crate::error::{OsError, Result};
use std::mem::MaybeUninit;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::{mem, ptr};
use windows_sys::Win32::Foundation;
use windows_sys::Win32::System::IO;

enum EventOp {
    Neutral,
    Pending(Waker),
    Ok(usize),
    Err(OsError),
}

pub(super) struct IocpEvent {
    op: EventOp,
}

impl IocpEvent {
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

pub type Event = Arc<Mutex<IocpEvent>>;

fn iocp_new() -> Result<Handle> {
    const NULL_HANDLE: Foundation::HANDLE = ptr::null_mut();

    unsafe {
        match IO::CreateIoCompletionPort(Foundation::INVALID_HANDLE_VALUE, ptr::null_mut(), 0, 0) {
            NULL_HANDLE => Err(OsError::last()),
            handle => Ok(Handle::from_raw_handle(handle)),
        }
    }
}

fn iocp_add<T>(iocp: &Handle, soc: &T, event: &Event)
where
    T: AsRawHandle,
{
    unsafe {
        IO::CreateIoCompletionPort(
            soc.as_raw_handle(),
            iocp.as_raw_handle(),
            Arc::into_raw(event.clone()) as usize,
            0,
        );
    }
}

fn iocp_poll(iocp: &Handle, timeout: u32) -> Result<(Result<usize>, Event)> {
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
            let event: Event = Arc::from_raw(ev.assume_init() as *mut Mutex<_>);
            Ok((Ok(len as usize), event))
        } else {
            let ev = ev.assume_init();
            if ev > 0 {
                let event: Event = Arc::from_raw(ev as *mut Mutex<_>);
                Ok((Err(OsError::last()), event))
            } else {
                Err(OsError::last())
            }
        }
    }
}

pub struct Iocp {
    iocp: Handle,
    pub(super) intr: Intr,
    intr_event: Event,
}

impl Iocp {
    pub(super) fn new() -> Result<Iocp> {
        let iocp = iocp_new()?;
        let intr = Intr::new()?;
        let intr_event: Event = Default::default();
        iocp_add(&iocp, intr.as_handle(), &intr_event);
        Ok(Iocp {
            iocp: iocp,
            intr: intr,
            intr_event: intr_event,
        })
    }

    pub(crate) fn add_socket<T>(&self, soc: &T, ev: &Event)
    where
        T: AsRawHandle,
    {
        iocp_add(&self.iocp, soc, ev)
    }

    pub(crate) fn del_socket<T>(&self, _soc: &T)
    where
        T: AsRawHandle,
    {
    }

    pub(super) fn poll(&self, scheduler: &EventScheduler) -> Poll<OsError> {
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
