use crate::buf::IoBuf;

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
