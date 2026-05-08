use super::MsgBuf;
use crate::socket_base::Endpoint;
use std::ops::{Deref, DerefMut};

pub struct MsgBufMut<'a>(&'a mut MsgBuf);

impl<'a> MsgBufMut<'a> {
    pub(super) fn new(mbuf: &'a mut MsgBuf) -> Self {
        Self(mbuf)
    }

    pub fn commit<E>(self, len: usize, ep: &E)
    where
        E: Endpoint,
    {
        self.0.commit(len, ep)
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.prepare_bytes()
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        self.0.prepare_bytes()
    }
}

impl<'a> Deref for MsgBufMut<'a> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.0.prepare_bytes()
    }
}

impl<'a> DerefMut for MsgBufMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.prepare_bytes()
    }
}
