use crate::buf::IoBuf;

/// # Safety
///
/// Buffers passed to IOCP operations must reference a stable memory
/// region. While the runtime holds ownership to a buffer, the pointer returned
/// by `as_buf_mut_ptr` must remain valid even if the `IoBufMut` value is moved.
pub unsafe trait IoBufMut: IoBuf {
    fn as_buf_mut_ptr(&mut self) -> *mut u8;
}

unsafe impl IoBufMut for Vec<u8> {
    fn as_buf_mut_ptr(&mut self) -> *mut u8 {
        self.as_mut_ptr()
    }
}

unsafe impl IoBufMut for &'static mut [u8] {
    fn as_buf_mut_ptr(&mut self) -> *mut u8 {
        self.as_mut_ptr()
    }
}
