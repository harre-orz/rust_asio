pub trait Buffer {
    fn buffer_size(&self) -> usize;
    fn as_buffer(&self) -> &[u8];
}

pub trait MutableBuffer : Buffer {
    fn as_mut_buffer(&mut self) -> &mut [u8];
}

impl<'a> Buffer for &'a [u8] {
    fn buffer_size(&self) -> usize {
        self.len()
    }

    fn as_buffer(&self) -> &[u8] {
        self
    }
}

impl<'a> Buffer for &'a mut [u8] {
    fn buffer_size(&self) -> usize {
        self.len()
    }

    fn as_buffer(&self) -> &[u8] {
        self
    }
}

impl<'a> MutableBuffer for &'a mut [u8] {
    fn as_mut_buffer(&mut self) -> &mut [u8] {
        self
    }
}

impl<'a> Buffer for &'a str {
    fn buffer_size(&self) -> usize {
        self.as_bytes().len()
    }

    fn as_buffer(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Buffer for String {
    fn buffer_size(&self) -> usize {
        self.as_bytes().len()
    }

    fn as_buffer(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Buffer for Vec<u8> {
    fn buffer_size(&self) -> usize {
        self.len()
    }

    fn as_buffer(&self) -> &[u8] {
        self.as_slice()
    }
}

impl MutableBuffer for Vec<u8> {
    fn as_mut_buffer(&mut self) -> &mut [u8] {
        self.as_mut_slice()
    }
}
