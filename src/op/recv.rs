use crate::{buf::*, op::*};
use windows_sys::Win32::Networking::WinSock::{WSARecv, WSABUF};

pub struct Recv<T: IoBufMut> {
    buffer: T,
}

impl<T: IoBufMut> Recv<T> {
    pub fn new(buffer: T) -> Self {
        Self { buffer }
    }
}

impl<T: IoBufMut> IocpOperation for Recv<T> {
    type Output = usize;
    type Buffer = T;

    unsafe fn operate(&mut self, handle: usize, overlapped_ptr: *mut OVERLAPPED) -> IoResult<()> {
        let buffer = WSABUF {
            len: self.buffer.buf_len() as _,
            buf: self.buffer.as_buf_mut_ptr() as _,
        };
        let mut flags = 0;
        let mut received = 0;
        let res = WSARecv(
            handle,
            &buffer,
            1,
            &mut received,
            &mut flags,
            overlapped_ptr,
            None,
        );
        wsa_result(res)
    }

    fn result(&mut self, res: usize) -> BufResult<Self::Output, Self::Buffer> {
        (Ok(res), self.buffer.take())
    }

    fn error(&mut self, err: IoError) -> BufResult<Self::Output, Self::Buffer> {
        (Err(err), self.buffer.take())
    }
}

pub struct RecvVectored<T: IoBufMut> {
    buffer: Vec<T>,
}

impl<T: IoBufMut> RecvVectored<T> {
    pub fn new(buffer: Vec<T>) -> Self {
        Self { buffer }
    }
}

impl<T: IoBufMut> IocpOperation for RecvVectored<T> {
    type Output = usize;
    type Buffer = Vec<T>;

    unsafe fn operate(&mut self, handle: usize, overlapped_ptr: *mut OVERLAPPED) -> IoResult<()> {
        let buffers = self
            .buffer
            .iter_mut()
            .map(|buf| WSABUF {
                len: buf.buf_len() as _,
                buf: buf.as_buf_mut_ptr() as _,
            })
            .collect::<Vec<_>>();
        let mut flags = 0;
        let mut received = 0;
        let res = WSARecv(
            handle,
            buffers.as_ptr(),
            buffers.len() as _,
            &mut received,
            &mut flags,
            overlapped_ptr,
            None,
        );
        wsa_result(res)
    }

    fn result(&mut self, res: usize) -> BufResult<Self::Output, Self::Buffer> {
        (Ok(res), std::mem::take(&mut self.buffer))
    }

    fn error(&mut self, err: IoError) -> BufResult<Self::Output, Self::Buffer> {
        (Err(err), std::mem::take(&mut self.buffer))
    }
}
