use core::AsIoContext;
use async::Handler;

pub trait Stream<E> : AsIoContext + Send + 'static {
    fn async_read_some<F>(&self, buf: &mut [u8], handler: F) -> F::Output
        where F: Handler<usize, E>;

    fn async_write_some<F>(&self, buf: &[u8], handler: F) -> F::Output
        where F: Handler<usize, E>;

    fn read_some(&self, buf: &mut [u8]) -> Result<usize, E>;

    fn write_some(&self, buf: &[u8]) -> Result<usize, E>;
}

mod match_cond;
pub use self::match_cond::*;

// pub fn read_until<S, M, E>(s: &S, sbuf: &mut StreamBuf, mut cond: M) -> Result<usize, E>
//     where S: Stream<E>,
//           M: MatchCondition,
//           E: From<io::Error>,
// {
//     let mut cur = 0;
//     loop {
//         match cond.match_cond(&sbuf.as_slice()[cur..]) {
//             Ok(len) => return Ok(cur + len),
//             Err(len) => {
//                 cur += len;
//                 let len = try!(s.read_some(try!(sbuf.prepare(4096))));
//                 sbuf.commit(len);
//             },
//         }
//     }
// }

// struct ReadUntilHandler<S, M, F, E> {
//     s: UnsafeRefCell<S>,
//     sbuf: UnsafeRefCell<StreamBuf>,
//     cond: M,
//     handler: F,
//     cur: usize,
//     _marker: PhantomData<E>,
// }

// impl<S, M, F, E> Handler<usize, E> for ReadUntilHandler<S, M, F, E>
//     where S: Stream<E>,
//           M: MatchCondition,
//           F: Handler<usize, E>,
//           E: From<io::Error> + Send + 'static,
// {
//     type Output = F::Output;

//     fn callback(self, io: &IoContext, res: Result<usize, E>) {
//         let ReadUntilHandler { s, mut sbuf, cond, handler, cur, _marker } = self;
//         let s = unsafe { s.as_ref() };
//         match res {
//             Ok(len) => {
//                 let sbuf = unsafe { sbuf.as_mut() };
//                 sbuf.commit(len);
//                 async_read_until_detail(s, sbuf, cond, handler, cur);
//             },
//             Err(err) => handler.callback(io, Err(err)),
//         }
//     }

//     fn wrap<G>(self, callback: G) -> Callback
//         where G: FnOnce(&IoContext, ErrCode, Self) + Send + 'static,
//     {
//         let ReadUntilHandler { s, sbuf, cond, handler, cur, _marker } = self;
//         handler.wrap(move |io, ec, handler| {
//             callback(io, ec, ReadUntilHandler {
//                 s: s,
//                 sbuf: sbuf,
//                 cond: cond,
//                 handler: handler,
//                 cur: cur,
//                 _marker: PhantomData,
//             })
//         })
//     }

//     type AsyncResult = F::AsyncResult;

//     fn async_result(&self) -> Self::AsyncResult {
//         self.handler.async_result()
//     }
// }

// fn async_read_until_detail<S, M, F, E>(s: &S, sbuf: &mut StreamBuf, mut cond: M, handler: F, mut cur: usize) -> F::Output
//     where S: Stream<E>,
//           M: MatchCondition,
//           F: Handler<usize, E>,
//           E: From<io::Error> + Send + 'static,
// {
//     let ctx = s.context();
//     let out = handler.async_result();
//     match cond.match_cond(&sbuf.as_slice()[cur..]) {
//         Ok(len) => handler.callback(ctx, Ok(cur + len)),
//         Err(len) => {
//             cur += len;
//             let sbuf_ptr = UnsafeRefCell::new(sbuf);
//             match sbuf.prepare(4096) {
//                 Ok(buf) => {
//                     let handler = ReadUntilHandler {
//                         s: UnsafeRefCell::new(s),
//                         sbuf: sbuf_ptr,
//                         cond: cond,
//                         handler: handler,
//                         cur: cur,
//                         _marker: PhantomData,
//                     };
//                     s.async_read_some(buf, handler);
//                 },
//                 Err(err) => handler.callback(ctx, Err(err.into())),
//             }
//         }
//     }
//     out.get(ctx)
// }

// pub fn async_read_until<S, M, F, E>(s: &S, sbuf: &mut StreamBuf, cond: M, handler: F) -> F::Output
//     where S: Stream<E>,
//           M: MatchCondition,
//           F: Handler<usize, E>,
//           E: From<io::Error> + Send + 'static,
// {
//     async_read_until_detail(s, sbuf, cond, handler, 0)
// }

// pub fn write_until<S, M, E>(s: &S, sbuf: &mut StreamBuf, mut cond: M) -> Result<usize, E>
//     where S: Stream<E>,
//           M: MatchCondition,
// {
//     let len = {
//         let len = match cond.match_cond(sbuf.as_slice()) {
//             Ok(len) => len,
//             Err(len) => len,
//         };
//         try!(s.write_some(&sbuf.as_slice()[..len]))
//     };
//     sbuf.consume(len);
//     Ok(len)
// }

// struct WriteUntilHandler<S, F, E> {
//     s: UnsafeRefCell<S>,
//     sbuf: UnsafeRefCell<StreamBuf>,
//     handler: F,
//     total: usize,
//     cur: usize,
//     _marker: PhantomData<E>,
// }

// impl<S, F, E> Handler<usize, E> for WriteUntilHandler<S, F, E>
//     where S: Stream<E>,
//           F: Handler<usize, E>,
//           E: Send + 'static,
// {
//     type Output = F::Output;

//     fn callback(self, io: &IoContext, res: Result<usize, E>) {
//         let WriteUntilHandler { s, mut sbuf, handler, total, mut cur, _marker } = self;
//         let s = unsafe { s.as_ref() };
//         match res {
//             Ok(len) => {
//                 let sbuf = unsafe { sbuf.as_mut() };
//                 sbuf.consume(len);
//                 cur -= len;
//                 if cur == 0 {
//                     handler.callback(io, Ok(total))
//                 } else {
//                     async_write_until_detail(s, sbuf, len, handler, cur);
//                 }
//             },
//             Err(err) => handler.callback(io, Err(err)),
//         }
//     }

//     fn wrap<G>(self, callback: G) -> Callback
//         where G: FnOnce(&IoContext, ErrCode, Self) + Send + 'static,
//     {
//         let WriteUntilHandler { s, sbuf, handler, total, cur, _marker } = self;
//         handler.wrap(move |io, ec, handler| {
//             callback(io, ec, WriteUntilHandler {
//                 s: s,
//                 sbuf: sbuf,
//                 handler: handler,
//                 total: total,
//                 cur: cur,
//                 _marker: _marker,
//             })
//         })
//     }

//     type AsyncResult = F::AsyncResult;

//     fn async_result(&self) -> Self::AsyncResult {
//         self.handler.async_result()
//     }
// }

// fn async_write_until_detail<S, F, E>(s: &S, sbuf: &mut StreamBuf, total: usize, handler: F, cur: usize) -> F::Output
//     where S: Stream<E>,
//           F: Handler<usize, E>,
//           E: Send + 'static,
// {
//     let handler = WriteUntilHandler {
//         s: UnsafeRefCell::new(s),
//         sbuf: UnsafeRefCell::new(sbuf),
//         handler: handler,
//         total: total,
//         cur: cur,
//         _marker: PhantomData,
//     };
//     s.async_write_some(&sbuf.as_slice()[..cur], handler)
// }

// pub fn async_write_until<S, M, F, E>(s: &S, sbuf: &mut StreamBuf, mut cond: M, handler: F) -> F::Output
//     where S: Stream<E>,
//           M: MatchCondition,
//           F: Handler<usize, E>,
//           E: From<io::Error> + Send + 'static,
// {
//     let total = match cond.match_cond(sbuf.as_slice()) {
//         Ok(len) => len,
//         Err(len) => len,
//     };
//     async_write_until_detail(s, sbuf, total, handler, total)
// }
