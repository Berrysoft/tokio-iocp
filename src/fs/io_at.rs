use crate::{io_port::OverlappedWaker, op::IocpOperation, *};
use std::{
    cell::OnceCell,
    ops::DerefMut,
    os::windows::prelude::{AsRawHandle, BorrowedHandle},
    pin::Pin,
    task::{Context, Poll},
};
use windows_sys::Win32::{
    Foundation::{GetLastError, ERROR_HANDLE_EOF, ERROR_IO_INCOMPLETE},
    System::IO::GetOverlappedResult,
};

pub struct FileAsyncIoAt<'a, Op: IocpOperation> {
    handle: BorrowedHandle<'a>,
    op: Op,
    overlapped_ptr: OnceCell<usize>,
}

impl<'a, Op: IocpOperation> FileAsyncIoAt<'a, Op> {
    pub(crate) fn new(handle: BorrowedHandle<'a>, op: Op) -> Self {
        Self {
            handle,
            op,
            overlapped_ptr: OnceCell::new(),
        }
    }
}

impl<Op: IocpOperation> Future for FileAsyncIoAt<'_, Op> {
    type Output = BufResult<Op::Output, Op::Buffer>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.deref_mut();
        let overlapped_ptr = match this.overlapped_ptr.get_or_try_init(|| {
            let mut overlapped = Box::new(OverlappedWaker::new());
            overlapped.set_waker(cx.waker().clone());
            let overlapped_ptr = overlapped.leak();
            unsafe {
                this.op
                    .operate(this.handle.as_raw_handle() as _, overlapped_ptr as _)?;
            }
            Ok(overlapped_ptr as usize)
        }) {
            Ok(ptr) => *ptr,
            Err(e) => return Poll::Ready(self.op.error(e)),
        };
        let mut transferred = 0;
        let res = unsafe {
            GetOverlappedResult(
                this.handle.as_raw_handle() as _,
                overlapped_ptr as _,
                &mut transferred,
                0,
            )
        };
        if res == 0 {
            let error = unsafe { GetLastError() };
            match error {
                ERROR_IO_INCOMPLETE => Poll::Pending,
                ERROR_HANDLE_EOF => Poll::Ready(self.op.result(0)),
                _ => Poll::Ready(self.op.error(IoError::from_raw_os_error(error as _))),
            }
        } else {
            Poll::Ready(self.op.result(transferred as _))
        }
    }
}
