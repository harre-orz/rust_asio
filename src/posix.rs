//

use executor::{IoContext, YieldContext};
use socket::{ioctl, NativeHandle,
             nb_read_some, nb_write_some,
             wa_read_some, wa_write_some,
};
use socket_base::{Socket, IoControl};
use stream::Stream;

use std::io;

struct Posix;

pub struct StreamDescriptor {
    ctx: IoContext,
    soc: NativeHandle,
}

impl Drop for StreamDescriptor {
    fn drop(&mut self) {
        let _ = self.ctx.disposal(self);
    }
}

impl StreamDescriptor {
    pub unsafe fn new(ctx: &IoContext, handle: NativeHandle) -> io::Result<Self> {
        Ok(Self::unsafe_new(ctx, Posix, handle))
    }

    pub fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub fn close(self) -> io::Result<()> {
        Ok(self.ctx.disposal(&self)?)
    }

    pub fn io_control<T>(&self, data: &mut T) -> io::Result<()>
        where T: IoControl,
    {
        Ok(ioctl(self.native_handle(), data)?)
    }
}

impl Socket<Posix> for StreamDescriptor {
    fn is_stopped(&self) -> bool {
        self.ctx.is_stopped()
    }

    fn native_handle(&self) -> NativeHandle {
        self.soc
    }

    unsafe fn unsafe_new(ctx: &IoContext, _: Posix, soc: NativeHandle) -> Self {
        ctx.placement(StreamDescriptor {
            ctx: ctx.clone(),
            soc: soc,
        })
    }
}

impl Stream for StreamDescriptor {
    type Error = io::Error;

    fn async_read_some(&self, buf: &mut [u8], yield_ctx: &mut YieldContext) -> Result<usize, Self::Error> {
        Ok(wa_read_some(self, buf, yield_ctx)?)
    }

    fn async_write_some(&self, buf: &[u8], yield_ctx: &mut YieldContext) -> Result<usize, Self::Error> {
        Ok(wa_write_some(self, buf, yield_ctx)?)
    }

    fn nb_read_some(&self, buf: &mut [u8]) -> io::Result<usize> {
        Ok(nb_read_some(self, buf)?)
    }

    fn nb_write_some(&self, buf: &[u8]) -> io::Result<usize> {
        Ok(nb_write_some(self, buf)?)
    }

    fn read_some(&self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let mut wait = self.as_ctx().blocking();
        Ok(wa_read_some(self, buf, &mut wait)?)
    }

    fn write_some(&self, buf: &[u8]) -> Result<usize, Self::Error> {
        let mut wait = self.as_ctx().blocking();
        Ok(wa_write_some(self, buf, &mut wait)?)
    }
}
