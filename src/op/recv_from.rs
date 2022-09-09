use crate::{net::*, op::*};
use std::marker::PhantomData;
use windows_sys::Win32::Networking::WinSock::WSARecvFrom;

pub struct RecvFrom<T: WithWsaBufMut, A: SockAddr> {
    buffer: T,
    addr_buffer: [u8; MAX_ADDR_SIZE],
    addr_size: i32,
    _marker: PhantomData<A>,
}

impl<T: WithWsaBufMut, A: SockAddr> RecvFrom<T, A> {
    pub fn new(buffer: T::Buffer) -> Self {
        Self {
            buffer: T::new(buffer),
            addr_buffer: [0; MAX_ADDR_SIZE],
            addr_size: MAX_ADDR_SIZE as _,
            _marker: PhantomData,
        }
    }
}

impl<T: WithWsaBufMut, A: SockAddr> IocpOperation for RecvFrom<T, A> {
    type Output = (usize, A);
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
        win32_result(res)
    }

    fn set_buf_len(&mut self, len: usize) {
        self.buffer.set_len(len)
    }

    fn result(&mut self, res: IoResult<usize>) -> BufResult<Self::Output, Self::Buffer> {
        let out = res.map(|res| {
            let addr =
                unsafe { A::try_from_native(self.addr_buffer.as_ptr() as _, self.addr_size as _) }
                    .unwrap();
            (res, addr)
        });
        (out, self.buffer.take_buf())
    }
}

pub type RecvFromOne<T, A> = RecvFrom<BufWrapper<T>, A>;
pub type RecvFromVectored<T, A> = RecvFrom<VectoredBufWrapper<T>, A>;
