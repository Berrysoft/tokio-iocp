use crate::{io_port::OverlappedWakerWrapper, op::IocpOperation, *};
use std::{
    os::windows::prelude::{AsRawHandle, BorrowedHandle},
    pin::Pin,
    task::{Context, Poll},
};
use windows_sys::Win32::{
    Foundation::{GetLastError, ERROR_HANDLE_EOF, ERROR_IO_INCOMPLETE},
    System::IO::GetOverlappedResult,
};

pub struct FileAsyncIo<'a, Op: IocpOperation> {
    handle: BorrowedHandle<'a>,
    op: Op,
    overlapped_ptr: OverlappedWakerWrapper,
}

impl<'a, Op: IocpOperation> FileAsyncIo<'a, Op> {
    pub(crate) fn new(handle: BorrowedHandle<'a>, op: Op) -> Self {
        Self {
            handle,
            op,
            overlapped_ptr: OverlappedWakerWrapper::new(),
        }
    }
}

impl<Op: IocpOperation> Future for FileAsyncIo<'_, Op> {
    type Output = BufResult<Op::Output, Op::Buffer>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let overlapped_ptr = match this
            .overlapped_ptr
            .get_and_try_op(cx.waker().clone(), |ptr| unsafe {
                this.op.operate(this.handle.as_raw_handle() as _, ptr)
            }) {
            Ok(ptr) => ptr,
            Err(e) => return Poll::Ready(this.op.error(e)),
        };
        let mut transferred = 0;
        let res = unsafe {
            GetOverlappedResult(
                this.handle.as_raw_handle() as _,
                overlapped_ptr,
                &mut transferred,
                0,
            )
        };
        if res == 0 {
            let error = unsafe { GetLastError() };
            match error {
                ERROR_IO_INCOMPLETE => Poll::Pending,
                ERROR_HANDLE_EOF => Poll::Ready(this.op.result(0)),
                _ => Poll::Ready(this.op.error(IoError::from_raw_os_error(error as _))),
            }
        } else {
            Poll::Ready(this.op.result(transferred as _))
        }
    }
}
