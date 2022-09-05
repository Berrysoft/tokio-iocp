use windows_sys::Win32::Networking::WinSock::{
    WSAGetLastError, WSAGetOverlappedResult, WSA_IO_INCOMPLETE,
};

use crate::{buf::*, io_port::OverlappedWaker, op::*, *};
use std::{
    ops::DerefMut,
    os::windows::prelude::{AsRawSocket, BorrowedSocket},
    pin::Pin,
    task::{Context, Poll},
};

pub struct SocketAsyncIo<'a, Op: IocpOperation> {
    handle: BorrowedSocket<'a>,
    buffer: Op::Buffer,
    op: Op,
    overlapped_ptr: usize,
}

impl<'a, Op: IocpOperation> SocketAsyncIo<'a, Op> {
    pub(crate) fn new(handle: BorrowedSocket<'a>, buffer: Op::Buffer, op: Op) -> Self {
        Self {
            handle,
            buffer,
            op,
            overlapped_ptr: 0,
        }
    }

    fn result(mut self: Pin<&mut Self>, res: IoResult<usize>) -> BufResult<usize, Op::Buffer> {
        (res, self.buffer.take())
    }
}

impl<Op: IocpOperation> Future for SocketAsyncIo<'_, Op> {
    type Output = BufResult<usize, Op::Buffer>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.buffer.buf_len() == 0 {
            Poll::Ready(self.result(Ok(0)))
        } else if self.overlapped_ptr == 0 {
            let mut overlapped = Box::new(OverlappedWaker::new());
            overlapped.set_waker(cx.waker().clone());
            let overlapped_ptr = overlapped.leak();
            let res = unsafe {
                let this = self.deref_mut();
                this.op.operate(
                    this.handle.as_raw_socket() as _,
                    &mut this.buffer,
                    overlapped_ptr as _,
                )
            };
            self.overlapped_ptr = overlapped_ptr as _;
            res.map(|res| self.result(res.map(|trans| trans as usize)))
        } else {
            let mut transferred = 0;
            let res = unsafe {
                let mut flags = 0;
                WSAGetOverlappedResult(
                    self.handle.as_raw_socket() as _,
                    self.overlapped_ptr as _,
                    &mut transferred,
                    0,
                    &mut flags,
                )
            };
            if res == 0 {
                let error = unsafe { WSAGetLastError() };
                match error {
                    WSA_IO_INCOMPLETE => Poll::Pending,
                    _ => Poll::Ready(self.result(Err(IoError::from_raw_os_error(error as _)))),
                }
            } else {
                self.overlapped_ptr = 0;
                Poll::Ready(self.result(Ok(transferred as _)))
            }
        }
    }
}
