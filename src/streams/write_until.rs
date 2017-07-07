use error::READY;
use core::workplace;
use async::{Handler, Receiver};
use buffers::StreamBuf;
use streams::{Stream, MatchCondition};
use reactive_io::{AsyncOutput};

use std::io;

pub fn write_until<S, M, E>(s: &S, sbuf: &mut StreamBuf, mut cond: M) -> Result<usize, E>
    where S: Stream<E>,
          M: MatchCondition,
{
    let len = {
        let len = match cond.match_cond(sbuf.as_slice()) {
            Ok(len) => len,
            Err(len) => len,
        };
        try!(s.write_some(&sbuf.as_slice()[..len]))
    };
    sbuf.consume(len);
    Ok(len)
}

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

struct WriteUntilHandler {
}

pub fn async_write_until<S, M, F, E>(s: &S, sbuf: &mut StreamBuf, mut cond: M, handler: F) -> F::Output
    where S: Stream<E>,
          M: MatchCondition,
          F: Handler<usize, E>,
          E: From<io::Error> + Send + 'static,
{
    let total = match cond.match_cond(sbuf.as_slice()) {
        Ok(len) => len,
        Err(len) => len,
    };
    let (op, res) = handler.channel(WriteUntilHandler {
    });
    workplace(s.as_ctx(), |this| s.add_op(this, op, READY));
    res.recv(s.as_ctx())
}
