use windows_sys::Win32::Networking::WinSock::WSABUF;

pub trait WithBuf: Unpin {
    type Buffer: Unpin;

    fn new(buffer: Self::Buffer) -> Self;
    fn with_buf<R>(&self, f: impl FnOnce(*const u8, usize) -> R) -> R;

    fn take_buf(&mut self) -> Self::Buffer;
}

pub trait WithBufMut: WithBuf {
    fn set_len(&mut self, len: usize);
    fn with_buf_mut<R>(&mut self, f: impl FnOnce(*mut u8, usize) -> R) -> R;
}

pub trait WithWsaBuf: WithBuf {
    fn with_wsa_buf<R>(&self, f: impl FnOnce(*const WSABUF, usize) -> R) -> R;
}

pub trait WithWsaBufMut: WithBufMut + WithWsaBuf {
    fn with_wsa_buf_mut<R>(&mut self, f: impl FnOnce(*const WSABUF, usize) -> R) -> R;
}
