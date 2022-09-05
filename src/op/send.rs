use crate::{buf::*, op::*, *};
use windows_sys::Win32::Networking::WinSock::{WSASend, WSABUF};

pub struct Send<T: IoBuf> {
    buffer: T,
}

impl<T: IoBuf> Send<T> {
    pub fn new(buffer: T) -> Self {
        Self { buffer }
    }
}

impl<T: IoBuf> IocpOperation for Send<T> {
    type Output = usize;
    type Buffer = T;

    unsafe fn operate(&mut self, handle: usize, overlapped_ptr: *mut OVERLAPPED) -> IoResult<()> {
        let buffer = WSABUF {
            len: self.buffer.buf_len() as _,
            buf: self.buffer.as_buf_ptr() as _,
        };
        let mut sent = 0;
        let res = WSASend(handle, &buffer, 1, &mut sent, 0, overlapped_ptr, None);
        wsa_result(res)
    }

    fn result(&mut self, res: usize) -> BufResult<Self::Output, Self::Buffer> {
        (Ok(res), self.buffer.take())
    }

    fn error(&mut self, err: IoError) -> BufResult<Self::Output, Self::Buffer> {
        (Err(err), self.buffer.take())
    }
}

pub struct SendVectored<T: IoBuf> {
    buffer: Vec<T>,
}

impl<T: IoBuf> SendVectored<T> {
    pub fn new(buffer: Vec<T>) -> Self {
        Self { buffer }
    }
}

impl<T: IoBuf> IocpOperation for SendVectored<T> {
    type Output = usize;
    type Buffer = Vec<T>;

    unsafe fn operate(&mut self, handle: usize, overlapped_ptr: *mut OVERLAPPED) -> IoResult<()> {
        let buffers = self
            .buffer
            .iter()
            .map(|buf| WSABUF {
                len: buf.buf_len() as _,
                buf: buf.as_buf_ptr() as _,
            })
            .collect::<Vec<_>>();
        let mut sent = 0;
        let res = WSASend(
            handle,
            buffers.as_ptr(),
            buffers.len() as _,
            &mut sent,
            0,
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
