#[derive(Clone)]
pub struct IoContext {
    _x: i32,
}

impl IoContext {
    pub fn new() -> Self {
        Self { _x: 0 }
    }

    pub fn is_stopped(&self) -> bool {
        true
    }
}
