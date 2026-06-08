use super::{Event, EventScheduler, Interrupter};
use crate::error::{OsError, Result};
use crate::socket::{AsHandle, Handle, Socket};
use std::ptr;
use std::task::Poll;
use windows_sys::Win32::Foundation;
use windows_sys::Win32::System::IO;

const NULL_HANDLE: Foundation::HANDLE = ptr::null_mut();

fn iocp_new() -> Result<Handle> {
    unsafe {
        match IO::CreateIoCompletionPort(Foundation::INVALID_HANDLE_VALUE, ptr::null_mut(), 0, 0) {
            NULL_HANDLE => Err(OsError::last()),
            handle => Ok(Handle::new_unchecked(handle)),
        }
    }
}

fn iocp_add<T>(iocp: &Handle, soc: &T, event: &Event)
where
    T: AsHandle,
{
    unsafe {
        let _ = IO::CreateIoCompletionPort(
            soc.as_raw_handle(),
            iocp.as_raw_handle(),
            event.as_raw_ptr() as usize,
            0,
        );
    }
}

fn iocp_poll(iocp: &Handle, timeout: u32) -> Result<(Result<usize>, Event)> {
    let mut bytes = 0;
    let mut ev = 0;
    let mut _ov = ptr::null_mut();
    unsafe {
        if IO::GetQueuedCompletionStatus(
            iocp.as_raw_handle(),
            &mut bytes,
            &mut ev,
            &mut _ov,
            timeout,
        ) > 0
        {
            let event = unsafe { Event::from_raw_ptr(ev as *mut Event) };
            Ok((Ok(bytes as usize), event))
        } else if ev > 0 {
            let err = OsError::last();
            let event = unsafe { Event::from_raw_ptr(ev as *mut Event) };
            Ok((Err(err), event))
        } else {
            Err(OsError::last())
        }
    }
}

pub struct Iocp {
    iocp: Handle,
    pub(crate) intr: Interrupter,
    intr_event: Event,
}

impl Iocp {
    pub fn new() -> Result<Iocp> {
        let iocp = iocp_new()?;
        let intr = Interrupter::new()?;
        let intr_event = Event::new();
        iocp_add(&iocp, intr.as_handle(), &intr_event);
        Ok(Iocp {
            iocp: iocp,
            intr: intr,
            intr_event: intr_event,
        })
    }

    pub(crate) fn register_soc(&self, soc: &Socket, ev: &Event) {
        iocp_add(&self.iocp, soc, ev)
    }

    pub(crate) fn deregister_soc(&self, soc: &Socket) {}

    pub(super) fn poll(&self, scheduler: &EventScheduler) -> Poll<OsError> {
        match iocp_poll(&self.iocp, self.intr.timeout().as_millis() as u32) {
            Err(err) => Poll::Ready(err),
            Ok((res, mut event)) => {
                if ptr::eq(&event, &self.intr_event) {
                    self.intr.update_event()
                } else {
                    event.ready(res);
                }
                Poll::Pending
            }
        }
    }
}
