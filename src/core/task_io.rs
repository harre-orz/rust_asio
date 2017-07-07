use core::IoContext;

use std::io;
use std::sync::Arc;

pub struct TaskIoContext;

impl TaskIoContext {
    pub fn new() -> io::Result<IoContext> {
        Ok(IoContext(Arc::new(TaskIoContext)))
    }
}
