use crate::op::*;
use windows_sys::Win32::Networking::WinSock::WSARecv;

pub struct Recv<T: WithWsaBufMut> {
    buffer: T,
}

impl<T: WithWsaBufMut> Recv<T> {
    pub fn new(buffer: T::Buffer) -> Self {
        Self {
            buffer: T::new(buffer),
        }
    }
}

impl<T: WithWsaBufMut> IocpOperation for Recv<T> {
    type Output = usize;
    type Buffer = T::Buffer;

    unsafe fn operate(&mut self, handle: usize, overlapped_ptr: *mut OVERLAPPED) -> IoResult<()> {
        let res = self.buffer.with_wsa_buf_mut(|ptr, len| {
            let mut flags = 0;
            let mut received = 0;
            WSARecv(
                handle,
                ptr,
                len as _,
                &mut received,
                &mut flags,
                overlapped_ptr,
                None,
            )
        });
        win32_result(res)
    }

    fn set_buf_init(&mut self, len: usize) {
        self.buffer.set_init(len)
    }

    fn result(&mut self, res: IoResult<usize>) -> BufResult<Self::Output, Self::Buffer> {
        (res, self.buffer.take_buf())
    }
}

pub type RecvOne<T> = Recv<BufWrapper<T>>;
pub type RecvVectored<T> = Recv<VectoredBufWrapper<T>>;
