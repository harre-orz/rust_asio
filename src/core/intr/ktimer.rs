use crate::error::OsError;

pub(in super::super) struct Ktimer;

impl Ktimer {
    pub const fn new() -> Result<Ktimer, OsError> {
        Ok(Ktimer)
    }

    pub const fn update_event(&self) {}
}