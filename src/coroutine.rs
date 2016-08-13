use std::mem;
use std::ptr;
use std::boxed::FnBox;
use context::{Context, Transfer};
use context::stack::ProtectedFixedSizeStack;
use {IoObject, IoService};

struct InitData {
    io: IoService,
    stack: ProtectedFixedSizeStack,
    callback: Box<FnBox(&mut Coroutine)>,
}

pub struct Coroutine {
    io: IoService,
    context: Option<Context>,
    result: usize,  // Box<Result> of yield_with
}

impl Coroutine {
    fn spawn<F: FnOnce(&mut Coroutine) + 'static>(io: IoService, f: F) -> Handle {
        let data = InitData {
            io: io,
            stack: ProtectedFixedSizeStack::default(),
            callback: Box::new(f),
        };

        let context = Context::new(&data.stack, Self::coroutine_entry);

        let mut data_opt = Some(data);
        let data_ptr = &mut data_opt as *mut _ as usize;
        let t = context.resume(data_ptr);

        let coro_ref = unsafe { &mut *(t.data as *mut Coroutine) };
        coro_ref.context = Some(t.context);

        Handle(coro_ref)
    }

    extern "C" fn coroutine_entry(t: Transfer) -> ! {
        let InitData { io, stack, callback } = unsafe {
            let data_opt_ref = &mut *(t.data as *mut Option<InitData>);
            data_opt_ref.take().unwrap()
        };

        let mut coro = Coroutine {
            io: io,
            context: None,
            result: 0,
        };

        let context = unsafe {
            let coro_ptr = &mut coro as *mut _ as usize;
            let Transfer { context, data:_ } = t.context.resume(coro_ptr);
            let coro_ref = &mut *(coro_ptr as *mut Coroutine);
            coro_ref.context = Some(context);

            callback.call_box((coro_ref,));
            coro_ref.context.take().unwrap()
        };

        let mut stack_opt = Some(stack);
        context.resume_ontop(&mut stack_opt as *mut _ as usize, Self::coroutine_exit);

        unreachable!();
    }

    extern "C" fn coroutine_exit(mut t: Transfer) -> Transfer {
        unsafe {
            // Drop the stack
            let stack_ref = &mut *(t.data as *mut Option<ProtectedFixedSizeStack>);
            let _ = stack_ref.take().unwrap();
            t.context = { mem::transmute(0usize) };
        }
        t
    }

    pub fn yield_with<F: FnOnce() -> R, R>(&mut self, function: F) -> R {
        let callback: Box<Box<FnBox() -> usize>> = Box::new(Box::new(move || {
            let output = Box::new(function());
            Box::into_raw(output) as usize
        }));
        debug_assert_eq!(mem::size_of_val(&callback), 8);

        let Transfer { context, data } = self.context.take().unwrap().resume(Box::into_raw(callback) as usize);
        self.context = Some(context);
        let output = unsafe { Box::from_raw(data as *mut R) };
        *output
    }
}

impl IoObject for Coroutine {
    fn io_service(&self) -> &IoService {
        &self.io
    }
}

struct Handle(*mut Coroutine);

impl Handle {
    fn is_finished(&self) -> bool {
        self.0 == ptr::null_mut()
    }

    fn resume(&mut self) {
        assert!(self.0 != ptr::null_mut());

        let coro = unsafe { &mut *self.0 };
        let Transfer { context, data } = coro.context.take().unwrap().resume(coro.result);
        let x: usize = unsafe { mem::transmute_copy(&context) };
        if x == 0 {
            self.0 = ptr::null_mut();
        } else {
            let callback: Box<Box<FnBox() -> usize>> = unsafe { Box::from_raw(data as *mut Box<FnBox() -> usize>) };
            coro.context = Some(context);
            coro.result = (*callback)();
        }
    }
}

unsafe impl Send for Handle {}

impl Drop for Handle {
    fn drop(&mut self) {
        assert!(self.is_finished());
    }
}

fn spawn_impl(io: &IoService, mut handle: Handle) {
    io.post(move |io| {
        handle.resume();
        if !handle.is_finished() {
            spawn_impl(io, handle);
        }
    });
}

pub fn spawn<T: IoObject, F: FnOnce(&mut Coroutine) + 'static>(io: &T, callback: F) {
    spawn_impl(io.io_service(), Coroutine::spawn(io.io_service().clone(), callback));
}


#[test]
fn test_coro() {
    let io = IoService::new();
    spawn(&io, move |coro| {
        assert_eq!(100, coro.yield_with(|| 100));
        assert_eq!(3.14, coro.yield_with(|| 3.14));
        assert_eq!("hello", coro.yield_with(|| "hello"));
    });
    io.run();
}
