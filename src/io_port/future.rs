use crate::{io_port::waker::*, op::IocpOperation, *};
use std::{
    future::Future,
    marker::PhantomData,
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

#[derive(Debug, Clone, Copy)]
enum IocpFutureState {
    NotPolled,
    Started,
    Finished,
}

pub struct IocpFuture<'a, Op: IocpOperation> {
    handle: BorrowedRes<'a>,
    overlapped: Rc<OverlappedWaker>,
    _p: PhantomData<Rc<Op>>,
    state: IocpFutureState,
}

impl<'a, Op: IocpOperation + 'static> IocpFuture<'a, Op> {
    pub fn new(handle: impl Into<BorrowedRes<'a>>, op: Op) -> Self {
        Self {
            handle: handle.into(),
            overlapped: Rc::new(OverlappedWaker::new(IoWakerOp::new(op))),
            _p: PhantomData,
            state: IocpFutureState::NotPolled,
        }
    }

    fn result(&mut self, res: IoResult<usize>) -> BufResult<Op::Output, Op::Buffer> {
        self.state = IocpFutureState::Finished;
        unsafe { self.overlapped.waker().op_mut::<Op>() }.result(res)
    }
}

impl<Op: IocpOperation + 'static> Future for IocpFuture<'_, Op> {
    type Output = BufResult<Op::Output, Op::Buffer>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };

        if let IocpFutureState::NotPolled = this.state {
            let overlapped_ptr =
                Rc::into_raw(this.overlapped.clone()) as *const OVERLAPPED as *mut OVERLAPPED;
            let res = unsafe {
                this.overlapped
                    .waker()
                    .op_mut::<Op>()
                    .operate(this.handle.as_raw_handle() as _, overlapped_ptr)
            };
            match res {
                Ok(()) => {
                    this.state = IocpFutureState::Started;
                }
                Err(e) => {
                    unsafe {
                        let _ = Rc::from_raw(overlapped_ptr as *mut OverlappedWaker);
                    }
                    return Poll::Ready(this.result(Err(e)));
                }
            }
        }

        // We need to set the recent waker.
        this.overlapped.waker().set_waker(cx.waker().clone());
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
            match this.overlapped.waker().take_err() {
                None => {
                    let transferred = transferred as usize;
                    unsafe { this.overlapped.waker().op_mut::<Op>() }.set_buf_init(transferred);
                    Poll::Ready(this.result(Ok(transferred)))
                }
                Some(err) => Poll::Ready(this.result(Err(err))),
            }
        }
    }
}

impl<Op: IocpOperation> Drop for IocpFuture<'_, Op> {
    fn drop(&mut self) {
        if let IocpFutureState::Finished = self.state {
            // Finished, no need to cancel.
        } else {
            self.overlapped.waker().take_waker();
            unsafe {
                CancelIoEx(
                    self.handle.as_raw_handle() as _,
                    self.overlapped.as_ref() as *const _ as *const OVERLAPPED,
                )
            };
        }
    }
}
