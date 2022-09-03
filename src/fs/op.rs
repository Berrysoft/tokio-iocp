use crate::*;
use std::{os::windows::prelude::RawHandle, task::Poll};
use windows_sys::Win32::{
    Foundation::{GetLastError, ERROR_IO_PENDING},
    Storage::FileSystem::{ReadFile, WriteFile},
    System::IO::OVERLAPPED,
};

pub trait IocpOperation {
    unsafe fn operate(
        handle: RawHandle,
        buffer: &mut [u8],
        overlapped_ptr: *mut OVERLAPPED,
    ) -> Poll<IoResult<u32>>;
}

unsafe fn retrieve_result(res: i32, transfered: u32) -> Poll<IoResult<u32>> {
    if res == 0 {
        let error = GetLastError();
        match error {
            ERROR_IO_PENDING => Poll::Pending,
            _ => Poll::Ready(Err(IoError::from_raw_os_error(error as _))),
        }
    } else {
        Poll::Ready(Ok(transfered))
    }
}

pub struct Read;

impl IocpOperation for Read {
    unsafe fn operate(
        handle: RawHandle,
        buffer: &mut [u8],
        overlapped_ptr: *mut OVERLAPPED,
    ) -> Poll<IoResult<u32>> {
        let mut read = 0;
        let res = ReadFile(
            handle as _,
            buffer.as_mut_ptr() as _,
            buffer.len() as _,
            &mut read,
            overlapped_ptr,
        );
        retrieve_result(res, read)
    }
}

pub struct Write;

impl IocpOperation for Write {
    unsafe fn operate(
        handle: RawHandle,
        buffer: &mut [u8],
        overlapped_ptr: *mut OVERLAPPED,
    ) -> Poll<IoResult<u32>> {
        let mut written = 0;
        let res = WriteFile(
            handle as _,
            buffer.as_ptr() as _,
            buffer.len() as _,
            &mut written,
            overlapped_ptr,
        );
        retrieve_result(res, written)
    }
}
