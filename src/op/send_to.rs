use crate::op::*;
use std::net::SocketAddr;
use windows_sys::Win32::Networking::WinSock::WSASendTo;

pub struct SendTo<T: WithWsaBuf> {
    buffer: T,
    addr: SocketAddr,
}

impl<T: WithWsaBuf> SendTo<T> {
    pub fn new(buffer: T::Buffer, addr: SocketAddr) -> Self {
        Self {
            buffer: T::new(buffer),
            addr,
        }
    }
}

impl<T: WithWsaBuf> IocpOperation for SendTo<T> {
    type Output = usize;
    type Buffer = T::Buffer;

    unsafe fn operate(&mut self, handle: usize, overlapped_ptr: *mut OVERLAPPED) -> IoResult<()> {
        let res = self.buffer.with_wsa_buf(|ptr, len| {
            let mut sent = 0;
            wsa_exact_addr(self.addr, |addr, addr_len| {
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
        wsa_result(res)
    }

    fn result(&mut self, res: usize) -> BufResult<Self::Output, Self::Buffer> {
        (Ok(res), self.buffer.take_buf())
    }

    fn error(&mut self, err: IoError) -> BufResult<Self::Output, Self::Buffer> {
        (Err(err), self.buffer.take_buf())
    }
}

pub type SendToOne<T> = SendTo<BufWrapper<T>>;
pub type SendToVectored<T> = SendTo<VectoredBufWrapper<T>>;
