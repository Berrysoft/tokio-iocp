use crate::{buf::*, op::*};
use std::marker::PhantomData;
use windows_sys::Win32::Networking::WinSock::{
    WSAGetLastError, WSARecv, WSASend, WSABUF, WSA_IO_INCOMPLETE,
};

unsafe fn retrieve_result(res: i32, transfered: u32) -> Poll<IoResult<u32>> {
    if res == 0 {
        let error = WSAGetLastError();
        match error {
            WSA_IO_INCOMPLETE => Poll::Pending,
            _ => Poll::Ready(Err(IoError::from_raw_os_error(error as _))),
        }
    } else {
        Poll::Ready(Ok(transfered))
    }
}

pub struct Recv<T: IoBufMut>(PhantomData<T>);

impl<T: IoBufMut> Default for Recv<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: IoBufMut> IocpOperation for Recv<T> {
    type Buffer = T;

    unsafe fn operate(
        &self,
        handle: usize,
        buffer: &mut Self::Buffer,
        overlapped_ptr: *mut OVERLAPPED,
    ) -> Poll<IoResult<u32>> {
        let buffer = WSABUF {
            len: buffer.buf_len() as _,
            buf: buffer.as_buf_mut_ptr() as _,
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
        retrieve_result(res, received)
    }
}

pub struct Send<T: IoBuf>(PhantomData<T>);

impl<T: IoBuf> Default for Send<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: IoBuf> IocpOperation for Send<T> {
    type Buffer = T;

    unsafe fn operate(
        &self,
        handle: usize,
        buffer: &mut Self::Buffer,
        overlapped_ptr: *mut OVERLAPPED,
    ) -> Poll<IoResult<u32>> {
        let buffer = WSABUF {
            len: buffer.buf_len() as _,
            buf: buffer.as_buf_ptr() as _,
        };
        let mut sent = 0;
        let res = WSASend(handle, &buffer, 1, &mut sent, 0, overlapped_ptr, None);
        retrieve_result(res, sent)
    }
}
