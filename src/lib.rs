// Copyright 2016 Haruhiko Uchida
// The software is released under the MIT license.
// http://opensource.org/licenses/mit-license.php

#![feature(fnbox, unboxed_closures, test)]

extern crate test;
extern crate libc;
extern crate time;
extern crate context;

use std::io;
use std::mem;
use std::sync::Arc;

mod backbone;
use backbone::{SHUT_RD, SHUT_WR, SHUT_RDWR, RawFd, AsRawFd};

/// Possible values which can be passed to the shutdown method.
pub enum Shutdown {
    /// Indicates that the reading portion of this socket should be shut down.
    Read = SHUT_RD as isize,

    /// Indicates that the writing portion of this socket should be shut down.
    Write = SHUT_WR as isize,

    /// Shut down both the reading and writing portions of this socket.
    Both = SHUT_RDWR as isize,
}

pub trait Endpoint : Clone + Send + 'static {
    type SockAddr;

    fn as_sockaddr(&self) -> &Self::SockAddr;

    fn as_mut_sockaddr(&mut self) -> &mut Self::SockAddr;

    fn size(&self) -> usize;

    fn resize(&mut self, size: usize);

    fn capacity(&self) -> usize;
}

pub trait Protocol : Eq + PartialEq + Clone + Send + 'static {
    type Endpoint : Endpoint;

    /// Returns a value suitable for passing as the domain argument.
    fn family_type(&self) -> i32;

    /// Returns a value suitable for passing as the type argument.
    fn socket_type(&self) -> i32;

    /// Returns a value suitable for passing as the protocol argument.
    fn protocol_type(&self) -> i32;
}

pub trait NonBlocking : Sized + AsRawFd {
    fn get_non_blocking(&self) -> io::Result<bool>;

    fn set_non_blocking(&self, on: bool) -> io::Result<()>;
}

pub trait IoControl {
    type Data;

    fn name(&self) -> i32;

    fn data(&mut self) -> &mut Self::Data;
}

pub trait SocketOption<P: Protocol> {
    type Data;

    fn level(&self, pro: &P) -> i32;

    fn name(&self, pro: &P) -> i32;
}

pub trait GetSocketOption<P: Protocol> : SocketOption<P> + Default {
    fn data_mut(&mut self) -> &mut Self::Data;

    fn resize(&mut self, _size: usize) {
    }
}

pub trait SetSocketOption<P: Protocol> : SocketOption<P> {
    fn data(&self) -> &Self::Data;

    fn size(&self)  -> usize {
        mem::size_of::<Self::Data>()
    }
}

/// Traits to the associated with `IoService`.
pub trait IoObject : Sized {
    /// Returns a `IoService` associated with this object.
    fn io_service(&self) -> &IoService;
}

pub trait FromRawFd<P: Protocol> : AsRawFd + Send + 'static {
    unsafe fn from_raw_fd<T: IoObject>(io: &T, pro: P, fd: RawFd) -> Self;
}

pub trait Handler<A, R> : Send + 'static {
    fn callback(self, io: &IoService, actor: &A, res: io::Result<R>);
}

mod io_service;
use io_service::IoServiceBase;

#[derive(Clone)]
pub struct IoService(Arc<IoServiceBase>);

mod stream;
pub use self::stream::*;

mod stream_socket;
pub use self::stream_socket::StreamSocket;

mod dgram_socket;
pub use self::dgram_socket::DgramSocket;

mod raw_socket;
pub use self::raw_socket::RawSocket;

mod seq_packet_socket;
pub use self::seq_packet_socket::SeqPacketSocket;

mod socket_listener;
pub use self::socket_listener::SocketListener;

mod handler;
pub use self::handler::{ArcHandler, bind};

mod strand;
pub use self::strand::{Strand, StrandHandler};

mod coroutine;
pub use self::coroutine::{Coroutine, spawn};

pub mod socket_base;

pub mod ip;

pub mod local;

mod clock;
pub use self::clock::*;

mod from_str;
