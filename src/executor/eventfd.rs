
use super::{Reactor, SocketContext, callback_intr};

pub struct Intr {
    efd: SocketContext,
}

impl Intr {
    pub fn new() -> Result<Self, ErrorCode> {
        let efd = unsafe { libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) };
        if efd < 0 {
            return Err(ErrorCode::last_error())
        }
        Ok(Intr {
            efd: SocketContext {
                handle: efd,
                callback: callback_intr,
            },
        })
    }

    pub fn startup(&self, reactor: &Reactor) {
        reactor.register_interrupter(&self.efd);
    }

    pub fn cleanup(&self, reactor: &Reactor) {
        reactor.deregister_interrupter(&self.efd);
    }

    pub fn interrupt(&self) {
        let buf = [1, 0, 0, 0, 0, 0, 0, 0];
        let _ = unsafe {
            libc::write(
                self.efd.handle,
                buf.as_ptr() as *const _,
                buf.len() as _,
            )
        };
    }
}
