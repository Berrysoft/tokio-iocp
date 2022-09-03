use crate::{io_port::OverlappedWaker, *};
use std::{
    os::windows::prelude::{AsRawHandle, BorrowedHandle},
    pin::Pin,
    task::{Context, Poll},
};
use windows_sys::Win32::{
    Foundation::{GetLastError, ERROR_HANDLE_EOF, ERROR_IO_INCOMPLETE, ERROR_IO_PENDING},
    Storage::FileSystem::ReadFile,
    System::IO::GetOverlappedResult,
};

pub struct FileAsyncRead<'a> {
    handle: BorrowedHandle<'a>,
    pos: u32,
    buffer: Vec<u8>,
    overlapped_ptr: usize,
}

impl<'a> FileAsyncRead<'a> {
    pub(crate) fn new(handle: BorrowedHandle<'a>, pos: u32, buffer: Vec<u8>) -> Self {
        Self {
            handle,
            pos,
            buffer,
            overlapped_ptr: 0,
        }
    }

    fn result(mut self: Pin<&mut Self>, res: IoResult<usize>) -> (IoResult<usize>, Vec<u8>) {
        (res, std::mem::take(&mut self.buffer))
    }
}

impl Future for FileAsyncRead<'_> {
    type Output = (IoResult<usize>, Vec<u8>);

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.buffer.is_empty() {
            Poll::Ready(self.result(Ok(0)))
        } else if self.overlapped_ptr == 0 {
            let mut overlapped = Box::new(OverlappedWaker::new());
            overlapped.overlapped.Anonymous.Anonymous.Offset = self.pos;
            overlapped.set_waker(cx.waker().clone());
            let overlapped_ptr = overlapped.leak();
            let mut read = 0;
            let res = unsafe {
                ReadFile(
                    self.handle.as_raw_handle() as _,
                    self.buffer.as_mut_ptr() as _,
                    self.buffer.len() as _,
                    &mut read,
                    overlapped_ptr as _,
                )
            };
            self.overlapped_ptr = overlapped_ptr as usize;
            if res == 0 {
                let error = unsafe { GetLastError() };
                match error {
                    ERROR_IO_PENDING => Poll::Pending,
                    _ => Poll::Ready(self.result(Err(IoError::from_raw_os_error(error as _)))),
                }
            } else {
                Poll::Ready(self.result(Ok(read as _)))
            }
        } else {
            let mut transferred = 0;
            let res = unsafe {
                GetOverlappedResult(
                    self.handle.as_raw_handle() as _,
                    self.overlapped_ptr as _,
                    &mut transferred,
                    0,
                )
            };
            if res == 0 {
                let error = unsafe { GetLastError() };
                match error {
                    ERROR_IO_INCOMPLETE => Poll::Pending,
                    ERROR_HANDLE_EOF => Poll::Ready(self.result(Ok(0))),
                    _ => Poll::Ready(self.result(Err(IoError::from_raw_os_error(error as _)))),
                }
            } else {
                self.overlapped_ptr = 0;
                self.pos += transferred;
                Poll::Ready(self.result(Ok(transferred as _)))
            }
        }
    }
}
