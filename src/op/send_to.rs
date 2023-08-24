use crate::{net::*, op::*};
use windows_sys::Win32::Networking::WinSock::WSASendTo;

pub struct SendTo<T: WithWsaBuf, A: SockAddr> {
    buffer: T,
    addr: A,
}

impl<T: WithWsaBuf, A: SockAddr> SendTo<T, A> {
    pub fn new(buffer: T::Buffer, addr: A) -> Self {
        Self {
            buffer: T::new(buffer),
            addr,
        }
    }
}

impl<T: WithWsaBuf, A: SockAddr> IocpOperation for SendTo<T, A> {
    type Output = usize;
    type Buffer = T::Buffer;

    unsafe fn operate(
        &mut self,
        handle: usize,
        overlapped_ptr: *mut OVERLAPPED,
    ) -> Poll<IoResult<()>> {
        let res = self.buffer.with_wsa_buf(|ptr, len| {
            let mut sent = 0;
            self.addr.with_native(|addr, addr_len| {
                WSASendTo(
                    handle,
                    ptr,
                    len as _,
                    &mut sent,
                    0,
                    addr,
                    addr_len,
                    overlapped_ptr,
                    None,
                )
            })
        });
        win32_result(res)
    }

    fn set_buf_init(&mut self, _len: usize) {}

    fn result(&mut self, res: IoResult<usize>) -> BufResult<Self::Output, Self::Buffer> {
        (res, self.buffer.take_buf())
    }
}

pub type SendToOne<T, A> = SendTo<BufWrapper<T>, A>;
pub type SendToVectored<T, A> = SendTo<VectoredBufWrapper<T>, A>;
