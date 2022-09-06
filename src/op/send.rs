use crate::op::*;
use windows_sys::Win32::Networking::WinSock::WSASend;

pub struct Send<T: WithWsaBuf> {
    buffer: T,
}

impl<T: WithWsaBuf> Send<T> {
    pub fn new(buffer: T::Buffer) -> Self {
        Self {
            buffer: T::new(buffer),
        }
    }
}

impl<T: WithWsaBuf> IocpOperation for Send<T> {
    type Output = usize;
    type Buffer = T::Buffer;

    unsafe fn operate(&mut self, handle: usize, overlapped_ptr: *mut OVERLAPPED) -> IoResult<()> {
        let res = self.buffer.with_wsa_buf(|ptr, len| {
            let mut sent = 0;
            WSASend(handle, ptr, len as _, &mut sent, 0, overlapped_ptr, None)
        });
        win32_result(res)
    }

    fn set_buf_len(&mut self, _len: usize) {}

    fn result(&mut self, res: usize) -> BufResult<Self::Output, Self::Buffer> {
        (Ok(res), self.buffer.take_buf())
    }

    fn error(&mut self, err: IoError) -> BufResult<Self::Output, Self::Buffer> {
        (Err(err), self.buffer.take_buf())
    }
}

pub type SendOne<T> = Send<BufWrapper<T>>;
pub type SendVectored<T> = Send<VectoredBufWrapper<T>>;
