pub struct InnerSignal {
    ctx: IoContext,
    fd: KqueueFd,
    signals: AtomicUsize,
}

impl InnerSignal {
    pub fn new(ctx: &IoContext) -> Box<Self> {
        let soc = Box::new(InnerSignal {
            ctx: ctx.clone(),
            fd: KqueueFd::signal(),
            signals: AtomicUsize::new(0),
        });
        ctx.as_reactor().register_signal(&soc.fd);
        soc
    }

    pub fn add_read_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, _: SystemError) {
        let _kq = this.as_ctx().as_reactor().mutex.lock().unwrap();
        unsafe { &mut *(&self.fd as *const _ as *mut KqueueFd) }
            .input
            .queue
            .push_back(op)
    }

    pub fn cancel(&self) {
        self.fd.cancel_ops(&self.ctx)
    }

    pub fn next_read_op(&self, _: &mut ThreadIoContext) {}

    pub fn add(&self, sig: Signal) -> Result<(), SystemError> {
        let old = 1 << (sig as i32 as usize);
        if self.signals.fetch_or(old, Ordering::SeqCst) & old != 0 {
            return Err(INVALID_ARGUMENT);
        }
        let kev = make_sig(&KqueueFdPtr(&self.fd), EV_ADD | EV_ENABLE, sig as i32);
        let mut sigmask = self.ctx.as_reactor().sigmask.lock().unwrap();
        unsafe {
            sigaddset(&mut *sigmask, sig as i32);
            sigprocmask(SIG_SETMASK, &mut *sigmask, ptr::null_mut());
            libc::kevent(
                self.ctx.as_reactor().kq,
                &kev,
                1,
                ptr::null_mut(),
                0,
                ptr::null(),
            );
        }
        Ok(())
    }

    pub fn remove(&self, sig: Signal) -> Result<(), SystemError> {
        let old = 1 << (sig as i32 as usize);
        if self.signals.fetch_and(!old, Ordering::SeqCst) & old == 0 {
            return Err(INVALID_ARGUMENT);
        }
        let kev = make_sig(&KqueueFdPtr(&self.fd), EV_DELETE, sig as i32);
        unsafe {
            libc::kevent(
                self.ctx.as_reactor().kq,
                &kev,
                1,
                ptr::null_mut(),
                0,
                ptr::null(),
            );
        }
        Ok(())
    }

    pub fn clear(&self) {
        for sig in 0..32 {
            let old = 1 << sig;
            if self.signals.fetch_and(!old, Ordering::SeqCst) & old != 0 {
                let kev = make_sig(&KqueueFdPtr(&self.fd), EV_DELETE, sig);
                unsafe {
                    libc::kevent(
                        self.ctx.as_reactor().kq,
                        &kev,
                        1,
                        ptr::null_mut(),
                        0,
                        ptr::null(),
                    );
                }
            }
        }
        debug_assert_eq!(self.signals.load(Ordering::Relaxed), 0);
    }
}

unsafe impl AsIoContext for InnerSignal {
    fn as_ctx(&self) -> &IoContext {
        if let Some(this) = ThreadIoContext::callstack(&self.ctx) {
            this.as_ctx()
        } else {
            &self.ctx
        }
    }
}

impl Drop for InnerSignal {
    fn drop(&mut self) {
        self.clear();
        self.ctx.as_reactor().deregister_signal(&self.fd)
    }
}
