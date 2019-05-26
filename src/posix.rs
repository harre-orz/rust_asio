//

use executor::{AsIoContext, IoContext};
use socket::{bk_read_some, close, NativeHandle, Socket, Timeout};
use std::io;

struct Posix;

pub struct StreamDescriptor {
    ctx: IoContext,
    soc: NativeHandle,
    timeout: Timeout,
}

impl StreamDescriptor {
    pub unsafe fn from_raw_fd(ctx: &IoContext, fd: NativeHandle) -> Self {
        Self::unsafe_new(ctx, Posix, fd)
    }

    pub fn read_some(&self, buf: &mut [u8]) -> io::Result<usize> {
        Ok(bk_read_some(self, buf, self.timeout)?)
    }
}

impl Drop for StreamDescriptor {
    fn drop(&mut self) {
        let _ = close(self);
    }
}

impl AsIoContext for StreamDescriptor {
    fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }
}

impl Socket<Posix> for StreamDescriptor {
    fn native_handle(&self) -> NativeHandle {
        self.soc
    }

    unsafe fn unsafe_new(ctx: &IoContext, _: Posix, soc: NativeHandle) -> Self {
        StreamDescriptor {
            ctx: ctx.clone(),
            soc: soc,
            timeout: Timeout::new(),
        }
    }
}
