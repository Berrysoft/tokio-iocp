use crate::op::*;
use windows_sys::Win32::Networking::WinSock::WSARecvFrom;

pub struct RecvFrom<T: WithWsaBufMut> {
    buffer: T,
    addr_buffer: [u8; MAX_ADDR_SIZE],
    addr_size: i32,
}

impl<T: WithWsaBufMut> RecvFrom<T> {
    pub fn new(buffer: T::Buffer) -> Self {
        Self {
            buffer: T::new(buffer),
            addr_buffer: [0; MAX_ADDR_SIZE],
            addr_size: MAX_ADDR_SIZE as _,
        }
    }
}

impl<T: WithWsaBufMut> IocpOperation for RecvFrom<T> {
    type Output = (usize, SocketAddr);
    type Buffer = T::Buffer;

    unsafe fn operate(&mut self, handle: usize, overlapped_ptr: *mut OVERLAPPED) -> IoResult<()> {
        let res = self.buffer.with_wsa_buf_mut(|ptr, len| {
            let mut flags = 0;
            let mut received = 0;
            WSARecvFrom(
                handle,
                ptr,
                len as _,
                &mut received,
                &mut flags,
                self.addr_buffer.as_mut_ptr() as _,
                &mut self.addr_size,
                overlapped_ptr,
                None,
            )
        });
        wsa_result(res)
    }

    fn set_buf_len(&mut self, len: usize) {
        self.buffer.set_len(len)
    }

    fn result(&mut self, res: usize) -> BufResult<Self::Output, Self::Buffer> {
        let addr = unsafe { wsa_get_addr(self.addr_buffer.as_ptr() as _, self.addr_size as _) };
        (Ok((res, addr)), self.buffer.take_buf())
    }

    fn error(&mut self, err: IoError) -> BufResult<Self::Output, Self::Buffer> {
        (Err(err), self.buffer.take_buf())
    }
}

pub type RecvFromOne<T> = RecvFrom<BufWrapper<T>>;
pub type RecvFromVectored<T> = RecvFrom<VectoredBufWrapper<T>>;
