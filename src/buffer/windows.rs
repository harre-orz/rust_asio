use crate::buffer::ReserveBufError;
use crate::socket_base::{Endpoint, EndpointRef};
use std::alloc::{Layout, LayoutError};
use std::mem::MaybeUninit;
use std::pin::Pin;
use std::{ptr, slice};

pub struct MsgBuf {}

pub struct MsgBufMut<'a>(&'a mut MsgBuf);
