use crate::{buf::IoBuf, io_port::OverlappedWaker, *};
use std::{
    os::windows::prelude::{AsRawHandle, BorrowedHandle},
    pin::Pin,
    task::{Context, Poll},
};
use windows_sys::Win32::{
    Foundation::{GetLastError, ERROR_HANDLE_EOF, ERROR_IO_INCOMPLETE},
    System::IO::GetOverlappedResult,
};

use super::op::IocpOperation;

pub struct FileAsyncIoAt<'a, Op: IocpOperation> {
    handle: BorrowedHandle<'a>,
    pos: u32,
    buffer: Op::Buffer,
    overlapped_ptr: usize,
}

impl<'a, Op: IocpOperation> FileAsyncIoAt<'a, Op> {
    pub(crate) fn new(handle: BorrowedHandle<'a>, pos: u32, buffer: Op::Buffer) -> Self {
        Self {
            handle,
            pos,
            buffer,
            overlapped_ptr: 0,
        }
    }

    fn result(mut self: Pin<&mut Self>, res: IoResult<usize>) -> (IoResult<usize>, Op::Buffer) {
        (res, self.buffer.take())
    }
}

impl<Op: IocpOperation> Future for FileAsyncIoAt<'_, Op> {
    type Output = (IoResult<usize>, Op::Buffer);

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.buffer.buf_len() == 0 {
            Poll::Ready(self.result(Ok(0)))
        } else if self.overlapped_ptr == 0 {
            let mut overlapped = Box::new(OverlappedWaker::new());
            overlapped.overlapped.Anonymous.Anonymous.Offset = self.pos;
            overlapped.set_waker(cx.waker().clone());
            let overlapped_ptr = overlapped.leak();
            let res = unsafe {
                Op::operate(
                    self.handle.as_raw_handle(),
                    &mut self.buffer,
                    overlapped_ptr as _,
                )
            };
            self.overlapped_ptr = overlapped_ptr as _;
            res.map(|res| self.result(res.map(|trans| trans as usize)))
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
