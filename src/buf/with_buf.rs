use windows_sys::Win32::Networking::WinSock::WSABUF;

pub trait WrapBuf {
    type Buffer;

    fn new(buffer: Self::Buffer) -> Self;
    fn take_buf(&mut self) -> Self::Buffer;
}

pub trait WrapBufMut {
    fn set_len(&mut self, len: usize);
}

pub trait WithBuf: WrapBuf {
    fn with_buf<R>(&self, f: impl FnOnce(*const u8, usize) -> R) -> R;
}

pub trait WithBufMut: WrapBufMut + WithBuf {
    fn with_buf_mut<R>(&mut self, f: impl FnOnce(*mut u8, usize) -> R) -> R;
}

pub trait WithWsaBuf: WrapBuf {
    fn with_wsa_buf<R>(&self, f: impl FnOnce(*const WSABUF, usize) -> R) -> R;
}

pub trait WithWsaBufMut: WrapBufMut + WithWsaBuf {
    fn with_wsa_buf_mut<R>(&mut self, f: impl FnOnce(*const WSABUF, usize) -> R) -> R;
}
