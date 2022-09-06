use crate::{io_port::OverlappedWakerWrapper, op::*, *};
use std::{
    os::windows::prelude::{AsRawSocket, BorrowedSocket},
    pin::Pin,
    task::{Context, Poll},
};
use windows_sys::Win32::Networking::WinSock::{
    WSAGetLastError, WSAGetOverlappedResult, WSA_IO_INCOMPLETE,
};

pub struct SocketFuture<'a, Op: IocpOperation> {
    handle: BorrowedSocket<'a>,
    op: Op,
    overlapped_ptr: OverlappedWakerWrapper,
}

impl<'a, Op: IocpOperation> SocketFuture<'a, Op> {
    pub(crate) fn new(handle: BorrowedSocket<'a>, op: Op) -> Self {
        Self {
            handle,
            op,
            overlapped_ptr: OverlappedWakerWrapper::new(),
        }
    }
}

impl<Op: IocpOperation> Future for SocketFuture<'_, Op> {
    type Output = BufResult<Op::Output, Op::Buffer>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let overlapped_ptr = match this
            .overlapped_ptr
            .get_and_try_op(cx.waker().clone(), |ptr| unsafe {
                this.op.operate(this.handle.as_raw_socket() as _, ptr)
            }) {
            Ok(ptr) => ptr,
            Err(e) => return Poll::Ready(this.op.error(e)),
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
                _ => Poll::Ready(this.op.error(IoError::from_raw_os_error(error as _))),
            }
        } else {
            let transferred = transferred as usize;
            this.op.set_buf_len(transferred);
            Poll::Ready(this.op.result(transferred))
        }
    }
}
