use crate::{buf::*, op::*};
use windows_sys::Win32::Networking::WinSock::{
    WSAGetLastError, WSARecv, WSASend, WSABUF, WSA_IO_INCOMPLETE,
};

unsafe fn retrieve_result(res: i32) -> IoResult<()> {
    if res == 0 {
        let error = WSAGetLastError();
        match error {
            WSA_IO_INCOMPLETE => Ok(()),
            _ => Err(IoError::from_raw_os_error(error as _)),
        }
    } else {
        Ok(())
    }
}

pub struct Recv<T: IoBufMut> {
    buffer: T,
}

impl<T: IoBufMut> Recv<T> {
    pub fn new(buffer: T) -> Self {
        Self { buffer }
    }
}

impl<T: IoBufMut> IocpOperation for Recv<T> {
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
        retrieve_result(res)
    }

    fn take_buffer(&mut self) -> Self::Buffer {
        self.buffer.take()
    }
}

pub struct Send<T: IoBuf> {
    buffer: T,
}

impl<T: IoBuf> Send<T> {
    pub fn new(buffer: T) -> Self {
        Self { buffer }
    }
}

impl<T: IoBuf> IocpOperation for Send<T> {
    type Buffer = T;

    unsafe fn operate(&mut self, handle: usize, overlapped_ptr: *mut OVERLAPPED) -> IoResult<()> {
        let buffer = WSABUF {
            len: self.buffer.buf_len() as _,
            buf: self.buffer.as_buf_ptr() as _,
        };
        let mut sent = 0;
        let res = WSASend(handle, &buffer, 1, &mut sent, 0, overlapped_ptr, None);
        retrieve_result(res)
    }

    fn take_buffer(&mut self) -> Self::Buffer {
        self.buffer.take()
    }
}
