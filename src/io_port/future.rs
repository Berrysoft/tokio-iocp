use crate::{io_port::waker::OverlappedWakerWrapper, op::IocpOperation, *};
use std::{
    future::Future,
    os::windows::prelude::{AsRawHandle, AsRawSocket, BorrowedHandle, BorrowedSocket},
    pin::Pin,
    task::{Context, Poll},
};
use windows_sys::Win32::{
    Foundation::{GetLastError, ERROR_HANDLE_EOF, ERROR_IO_INCOMPLETE},
    System::IO::GetOverlappedResult,
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
    overlapped: OverlappedWakerWrapper,
}

impl<'a, Op: IocpOperation> IocpFuture<'a, Op> {
    pub fn new(handle: impl Into<BorrowedRes<'a>>, op: Op) -> Self {
        Self {
            handle: handle.into(),
            op,
            overlapped: OverlappedWakerWrapper::new(),
        }
    }
}

impl<Op: IocpOperation> Future for IocpFuture<'_, Op> {
    type Output = BufResult<Op::Output, Op::Buffer>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let (overlapped, overlapped_ptr) = match this
            .overlapped
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
            match overlapped.take_err() {
                None => {
                    let transferred = transferred as usize;
                    this.op.set_buf_len(transferred);
                    Poll::Ready(this.op.result(transferred))
                }
                Some(err) => Poll::Ready(this.op.error(err)),
            }
        }
    }
}
