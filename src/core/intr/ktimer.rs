use crate::error::Result;

pub(in super::super) struct Ktimer {}

impl Ktimer {
    pub fn new() -> Result<(Self, ())> {
        Ok((Ktimer {}, ()))
    }
}
