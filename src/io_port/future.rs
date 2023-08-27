use crate::{io_port::waker::*, *};
use std::{
    future::Future,
    os::windows::prelude::{AsRawHandle, AsRawSocket, BorrowedHandle, BorrowedSocket},
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
};
use windows_sys::Win32::{
    Foundation::{GetLastError, ERROR_HANDLE_EOF, ERROR_IO_INCOMPLETE},
    System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED},
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

pub struct IocpFuture<'a, T> {
    handle: BorrowedRes<'a>,
    result: Option<Poll<IoResult<()>>>,
    overlapped: Rc<OverlappedWaker<T>>,
}

impl<'a, T> IocpFuture<'a, T> {
    pub fn new(
        handle: impl Into<BorrowedRes<'a>>,
        buffer: T,
        op: impl FnOnce(usize, *mut OVERLAPPED, &mut T) -> Poll<IoResult<()>>,
    ) -> Self {
        let handle = handle.into();
        let overlapped = Rc::new(OverlappedWaker::new(buffer));
        let overlapped_ptr =
            Rc::into_raw(overlapped.clone()) as *const OVERLAPPED as *mut OVERLAPPED;
        let result = {
            let mut buffer = overlapped.buffer_mut();
            op(
                handle.as_raw_handle(),
                overlapped_ptr,
                buffer.as_mut().unwrap(),
            )
        };
        if result.is_ready() {
            unsafe { Rc::from_raw(overlapped_ptr as *mut OverlappedWaker<T>) };
        }
        Self {
            handle,
            result: Some(result),
            overlapped,
        }
    }

    fn result(&mut self, res: IoResult<usize>) -> BufResult<usize, T> {
        (res, self.overlapped.take_buffer())
    }
}

impl<T> Future for IocpFuture<'_, T> {
    type Output = BufResult<usize, T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };

        if let Some(result) = this.result.take() {
            // We need to set the recent waker.
            this.overlapped.set_waker(cx.waker().clone());
            match result {
                Poll::Pending | Poll::Ready(Ok(())) => {
                    let overlapped_ptr = this.overlapped.as_ref() as *const _ as *const OVERLAPPED;
                    let mut transferred = 0;
                    let res = unsafe {
                        GetOverlappedResult(
                            this.handle.as_raw_handle() as _,
                            overlapped_ptr,
                            &mut transferred,
                            0,
                        )
                    };
                    if matches!(result, Poll::Pending) && res == 0 {
                        let error = unsafe { GetLastError() };
                        match error {
                            ERROR_IO_INCOMPLETE => {
                                cx.waker().wake_by_ref();
                                this.result = Some(Poll::Pending);
                                Poll::Pending
                            }
                            ERROR_HANDLE_EOF => Poll::Ready(this.result(Ok(0))),
                            _ => Poll::Ready(
                                this.result(Err(IoError::from_raw_os_error(error as _))),
                            ),
                        }
                    } else {
                        let err = this.overlapped.take_err();
                        match err {
                            None => {
                                let transferred = transferred as usize;
                                Poll::Ready(this.result(Ok(transferred)))
                            }
                            Some(err) => Poll::Ready(this.result(Err(err))),
                        }
                    }
                }
                Poll::Ready(Err(e)) => Poll::Ready(this.result(Err(e))),
            }
        } else {
            unreachable!()
        }
    }
}

impl<T> Drop for IocpFuture<'_, T> {
    fn drop(&mut self) {
        if let Some(Poll::Pending) = self.result.take() {
            self.overlapped.take_waker();
            unsafe {
                CancelIoEx(
                    self.handle.as_raw_handle() as _,
                    self.overlapped.as_ref() as *const _ as *const OVERLAPPED,
                )
            };
        }
    }
}
