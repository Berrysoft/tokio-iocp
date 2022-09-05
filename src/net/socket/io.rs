use crate::{io_port::OverlappedWaker, op::*, *};
use std::{
    cell::OnceCell,
    ops::DerefMut,
    os::windows::prelude::{AsRawSocket, BorrowedSocket},
    pin::Pin,
    task::{Context, Poll},
};
use windows_sys::Win32::Networking::WinSock::{
    WSAGetLastError, WSAGetOverlappedResult, WSA_IO_INCOMPLETE,
};

pub struct SocketAsyncIo<'a, Op: IocpOperation> {
    handle: BorrowedSocket<'a>,
    op: Op,
    overlapped_ptr: OnceCell<usize>,
}

impl<'a, Op: IocpOperation> SocketAsyncIo<'a, Op> {
    pub(crate) fn new(handle: BorrowedSocket<'a>, op: Op) -> Self {
        Self {
            handle,
            op,
            overlapped_ptr: OnceCell::new(),
        }
    }

    fn result(mut self: Pin<&mut Self>, res: IoResult<usize>) -> BufResult<usize, Op::Buffer> {
        (res, self.op.take_buffer())
    }
}

impl<Op: IocpOperation> Future for SocketAsyncIo<'_, Op> {
    type Output = BufResult<usize, Op::Buffer>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.deref_mut();
        let overlapped_ptr = match this.overlapped_ptr.get_or_try_init(|| {
            let mut overlapped = Box::new(OverlappedWaker::new());
            overlapped.set_waker(cx.waker().clone());
            let overlapped_ptr = overlapped.leak();
            unsafe {
                this.op
                    .operate(this.handle.as_raw_socket() as _, overlapped_ptr as _)?;
            }
            Ok(overlapped_ptr as usize)
        }) {
            Ok(ptr) => *ptr,
            Err(e) => return Poll::Ready(self.result(Err(e))),
        };
        let mut transferred = 0;
        let res = unsafe {
            let mut flags = 0;
            WSAGetOverlappedResult(
                this.handle.as_raw_socket() as _,
                overlapped_ptr as _,
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
            Poll::Ready(self.result(Ok(transferred as _)))
        }
    }
}
