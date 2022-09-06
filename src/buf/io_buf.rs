/// # Safety
///
/// Buffers passed to IOCP operations must reference a stable memory
/// region. While the runtime holds ownership to a buffer, the pointer returned
/// by `as_buf_ptr` must remain valid even if the `IoBuf` value is moved.
pub unsafe trait IoBuf: Unpin + 'static {
    fn as_buf_ptr(&self) -> *const u8;
    fn buf_len(&self) -> usize;
    fn buf_capacity(&self) -> usize;
}

unsafe impl IoBuf for Vec<u8> {
    fn as_buf_ptr(&self) -> *const u8 {
        self.as_ptr()
    }

    fn buf_len(&self) -> usize {
        self.len()
    }

    fn buf_capacity(&self) -> usize {
        self.capacity()
    }
}

unsafe impl IoBuf for &'static mut [u8] {
    fn as_buf_ptr(&self) -> *const u8 {
        self.as_ptr()
    }

    fn buf_len(&self) -> usize {
        self.len()
    }

    fn buf_capacity(&self) -> usize {
        self.len()
    }
}

unsafe impl IoBuf for &'static [u8] {
    fn as_buf_ptr(&self) -> *const u8 {
        self.as_ptr()
    }

    fn buf_len(&self) -> usize {
        self.len()
    }

    fn buf_capacity(&self) -> usize {
        self.len()
    }
}

unsafe impl IoBuf for String {
    fn as_buf_ptr(&self) -> *const u8 {
        self.as_ptr()
    }

    fn buf_len(&self) -> usize {
        self.len()
    }

    fn buf_capacity(&self) -> usize {
        self.capacity()
    }
}

unsafe impl IoBuf for &'static mut str {
    fn as_buf_ptr(&self) -> *const u8 {
        self.as_ptr()
    }

    fn buf_len(&self) -> usize {
        self.len()
    }

    fn buf_capacity(&self) -> usize {
        self.len()
    }
}

unsafe impl IoBuf for &'static str {
    fn as_buf_ptr(&self) -> *const u8 {
        self.as_ptr()
    }

    fn buf_len(&self) -> usize {
        self.len()
    }

    fn buf_capacity(&self) -> usize {
        self.len()
    }
}

/// # Safety
///
/// Buffers passed to IOCP operations must reference a stable memory
/// region. While the runtime holds ownership to a buffer, the pointer returned
/// by `as_buf_mut_ptr` must remain valid even if the `IoBufMut` value is moved.
pub unsafe trait IoBufMut: IoBuf {
    fn as_buf_mut_ptr(&mut self) -> *mut u8;
    fn set_buf_len(&mut self, len: usize);
}

unsafe impl IoBufMut for Vec<u8> {
    fn as_buf_mut_ptr(&mut self) -> *mut u8 {
        self.as_mut_ptr()
    }

    fn set_buf_len(&mut self, len: usize) {
        if len > self.buf_len() {
            unsafe { self.set_len(len) };
        }
    }
}

unsafe impl IoBufMut for &'static mut [u8] {
    fn as_buf_mut_ptr(&mut self) -> *mut u8 {
        self.as_mut_ptr()
    }

    fn set_buf_len(&mut self, len: usize) {
        assert!(len <= self.buf_capacity())
    }
}
