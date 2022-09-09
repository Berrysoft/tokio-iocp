use crate::buf::*;
use std::ops::{Deref, DerefMut};

pub struct Slice<T> {
    buffer: T,
    begin: usize,
    end: usize,
}

impl<T> Slice<T> {
    pub(crate) fn new(buffer: T, begin: usize, end: usize) -> Self {
        Self { buffer, begin, end }
    }

    pub fn begin(&self) -> usize {
        self.begin
    }

    pub fn end(&self) -> usize {
        self.end
    }

    pub fn as_inner(&self) -> &T {
        &self.buffer
    }

    pub fn as_inner_mut(&mut self) -> &mut T {
        &mut self.buffer
    }

    pub fn into_inner(self) -> T {
        self.buffer
    }
}

fn deref<T: IoBuf>(buffer: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts(buffer.as_buf_ptr(), buffer.buf_len()) }
}

fn deref_mut<T: IoBufMut>(buffer: &mut T) -> &mut [u8] {
    unsafe { std::slice::from_raw_parts_mut(buffer.as_buf_mut_ptr(), buffer.buf_len()) }
}

impl<T: IoBuf> Deref for Slice<T> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        let bytes = deref(&self.buffer);
        let end = self.end.min(bytes.len());
        &bytes[self.begin..end]
    }
}

impl<T: IoBufMut> DerefMut for Slice<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let bytes = deref_mut(&mut self.buffer);
        let end = self.end.min(bytes.len());
        &mut bytes[self.begin..end]
    }
}

unsafe impl<T: IoBuf> IoBuf for Slice<T> {
    fn as_buf_ptr(&self) -> *const u8 {
        deref(&self.buffer)[self.begin..].as_ptr()
    }

    fn buf_len(&self) -> usize {
        self.deref().len()
    }

    fn buf_capacity(&self) -> usize {
        self.end - self.begin
    }
}

unsafe impl<T: IoBufMut> IoBufMut for Slice<T> {
    fn as_buf_mut_ptr(&mut self) -> *mut u8 {
        deref_mut(&mut self.buffer)[self.begin..].as_mut_ptr()
    }

    fn set_buf_len(&mut self, len: usize) {
        self.buffer.set_buf_len(self.begin + len)
    }
}
