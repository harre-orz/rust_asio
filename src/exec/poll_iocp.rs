use super::{Event, EventScheduler, Interrupter};
use crate::error::{OsError, Result};
use crate::socket::Socket;
use std::ptr;
use std::task::Poll;
use windows_sys::Win32::Foundation;
use windows_sys::Win32::System::IO;

const NULL_HANDLE: Foundation::HANDLE = ptr::null_mut();

fn iocp_new() -> Result<Foundation::HANDLE> {
    unsafe {
        match IO::CreateIoCompletionPort(Foundation::INVALID_HANDLE_VALUE, ptr::null_mut(), 0, 0) {
            NULL_HANDLE => Err(OsError::last()),
            handle => Ok(handle),
        }
    }
}

fn iocp_add(iocp: Foundation::HANDLE, handle: Foundation::HANDLE, event: &Event) {
    unsafe {
        let _ = IO::CreateIoCompletionPort(handle, iocp, event.as_raw_ptr() as usize, 0);
    }
}

fn iocp_poll(iocp: Foundation::HANDLE, timeout: u32) -> Result<(Result<usize>, Event)>{
    let mut bytes = 0;
    let mut ev = 0;
    let mut _ov = ptr::null_mut();
    unsafe {
        if IO::GetQueuedCompletionStatus(iocp, &mut bytes, &mut ev, &mut _ov, timeout) {
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
    iocp: Foundation::HANDLE,
    pub(crate) intr: Interrupter,
}

impl Iocp {
    pub fn new() -> Result<Iocp> {
        let iocp = iocp_new()?;
        let intr = Interrupter::new()?;
        Ok(Iocp {
            iocp: iocp,
            intr: intr,
        })
    }

    pub(crate) fn register_soc(&self, soc: &Socket, ev: &Event) {
        unsafe { iocp_add(self.iocp, soc.as_raw_socket() as Foundation::HANDLE, ev) }
    }

    pub(crate) fn deregister_soc(&self, soc: &Socket) {}

    pub(super) fn poll(&self, scheduler: &EventScheduler) -> Poll<OsError> {
        match iocp_poll(self.iocp, self.intr.timeout().as_millis() as u32) {
            Err(err) =>
                Poll::Ready(err),
            Ok((res, mut event)) => {
                event.ready(res);
                Poll::Pending
            },
        }
    }
}
