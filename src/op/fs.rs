use crate::{buf::*, op::*, *};
use std::{marker::PhantomData, task::Poll};
use windows_sys::Win32::{
    Foundation::{ERROR_HANDLE_EOF, ERROR_IO_INCOMPLETE},
    Storage::FileSystem::{ReadFile, WriteFile},
    System::IO::OVERLAPPED,
};

unsafe fn retrieve_result(res: i32, transfered: u32) -> Poll<IoResult<u32>> {
    if res == 0 {
        let error = GetLastError();
        match error {
            ERROR_IO_PENDING | ERROR_IO_INCOMPLETE => Poll::Pending,
            ERROR_HANDLE_EOF => Poll::Ready(Ok(0)),
            _ => Poll::Ready(Err(IoError::from_raw_os_error(error as _))),
        }
    } else {
        Poll::Ready(Ok(transfered))
    }
}

pub struct Read<T: IoBufMut>(PhantomData<T>);

impl<T: IoBufMut> Default for Read<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: IoBufMut> IocpOperation for Read<T> {
    type Buffer = T;

    unsafe fn operate(
        &self,
        handle: usize,
        buffer: &mut T,
        overlapped_ptr: *mut OVERLAPPED,
    ) -> Poll<IoResult<u32>> {
        let mut read = 0;
        let res = ReadFile(
            handle as _,
            buffer.as_buf_mut_ptr() as _,
            buffer.buf_len() as _,
            &mut read,
            overlapped_ptr,
        );
        retrieve_result(res, read)
    }
}

pub struct Write<T: IoBuf>(PhantomData<T>);

impl<T: IoBuf> Default for Write<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: IoBuf> IocpOperation for Write<T> {
    type Buffer = T;

    unsafe fn operate(
        &self,
        handle: usize,
        buffer: &mut T,
        overlapped_ptr: *mut OVERLAPPED,
    ) -> Poll<IoResult<u32>> {
        let mut written = 0;
        let res = WriteFile(
            handle as _,
            buffer.as_buf_ptr() as _,
            buffer.buf_len() as _,
            &mut written,
            overlapped_ptr,
        );
        retrieve_result(res, written)
    }
}
