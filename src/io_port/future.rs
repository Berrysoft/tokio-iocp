use crate::{io_port::waker::OverlappedWaker, op::IocpOperation, *};
use once_cell::unsync::OnceCell;
use std::{
    future::Future,
    os::windows::prelude::{AsRawHandle, AsRawSocket, BorrowedHandle, BorrowedSocket},
    pin::Pin,
    task::{Context, Poll},
};
use windows_sys::Win32::{
    Foundation::{GetLastError, ERROR_HANDLE_EOF, ERROR_IO_INCOMPLETE},
    System::IO::{CancelIoEx, GetOverlappedResult},
};

pub enum BorrowedRes<'a> {
    Handle(BorrowedHandle<'a>),
    Socket(BorrowedSocket<'a>),
}

impl BorrowedRes<'_> {
    pub fn as_raw_handle(&self) -> usize {
        match self {
            Self::Handle(h) => h.as_raw_handle() as _,
            Self::Socket(h) => h.as_raw_socket() as _,
        }
    }
}

impl<'a> From<BorrowedHandle<'a>> for BorrowedRes<'a> {
    fn from(h: BorrowedHandle<'a>) -> Self {
        Self::Handle(h)
    }
}

impl<'a> From<BorrowedSocket<'a>> for BorrowedRes<'a> {
    fn from(h: BorrowedSocket<'a>) -> Self {
        Self::Socket(h)
    }
}

pub struct IocpFuture<'a, Op: IocpOperation> {
    handle: BorrowedRes<'a>,
    op: Op,
    overlapped: OnceCell<Box<OverlappedWaker>>,
}

impl<'a, Op: IocpOperation> IocpFuture<'a, Op> {
    pub fn new(handle: impl Into<BorrowedRes<'a>>, op: Op) -> Self {
        Self {
            handle: handle.into(),
            op,
            overlapped: OnceCell::new(),
        }
    }

    fn result(&mut self, res: IoResult<usize>) -> BufResult<Op::Output, Op::Buffer> {
        _ = self.overlapped.take();
        self.op.result(res)
    }
}

impl<Op: IocpOperation> Future for IocpFuture<'_, Op> {
    type Output = BufResult<Op::Output, Op::Buffer>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        let overlapped = match this.overlapped.get_or_try_init(|| {
            let overlapped = Box::new(OverlappedWaker::new(cx.waker().clone()));
            unsafe {
                this.op
                    .operate(this.handle.as_raw_handle() as _, overlapped.as_ptr() as _)?;
            }
            Ok(overlapped)
        }) {
            Ok(o) => o,
            Err(e) => return Poll::Ready(this.result(Err(e))),
        };
        // We need to set the recent waker.
        overlapped.set_waker(cx.waker().clone());
        let overlapped_ptr = overlapped.as_ptr();
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
                ERROR_IO_INCOMPLETE => {
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
                ERROR_HANDLE_EOF => Poll::Ready(this.result(Ok(0))),
                _ => Poll::Ready(this.result(Err(IoError::from_raw_os_error(error as _)))),
            }
        } else {
            match overlapped.take_err() {
                None => {
                    let transferred = transferred as usize;
                    this.op.set_buf_len(transferred);
                    Poll::Ready(this.result(Ok(transferred)))
                }
                Some(err) => Poll::Ready(this.result(Err(err))),
            }
        }
    }
}

impl<Op: IocpOperation> Drop for IocpFuture<'_, Op> {
    fn drop(&mut self) {
        if let Some(overlapped) = self.overlapped.get() {
            unsafe { CancelIoEx(self.handle.as_raw_handle() as _, overlapped.as_ptr()) };
        }
    }
}
