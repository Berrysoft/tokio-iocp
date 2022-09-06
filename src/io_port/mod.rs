mod future;
pub use future::IocpFuture;
mod waker;

use crate::*;
use std::ptr::null_mut;
use windows_sys::Win32::{
    Foundation::{CloseHandle, GetLastError, ERROR_HANDLE_EOF, INVALID_HANDLE_VALUE, WAIT_TIMEOUT},
    System::IO::{CreateIoCompletionPort, GetQueuedCompletionStatus},
};

thread_local! {
    pub static IO_PORT: IoPort = IoPort::new().unwrap();
}

#[derive(Debug)]
pub struct IoPort {
    port: isize,
}

impl IoPort {
    pub fn new() -> IoResult<Self> {
        let port = unsafe { CreateIoCompletionPort(INVALID_HANDLE_VALUE, 0, 0, 0) };
        if port == 0 {
            Err(IoError::last_os_error())
        } else {
            Ok(Self { port })
        }
    }

    pub fn attach(&self, handle: usize) -> IoResult<()> {
        let port = unsafe { CreateIoCompletionPort(handle as isize, self.port, 0, 0) };
        if port == 0 {
            Err(IoError::last_os_error())
        } else {
            Ok(())
        }
    }

    pub fn poll(&self) {
        let mut transferred = 0;
        let mut key = 0;
        let mut overlapped_ptr = null_mut();
        let res = unsafe {
            GetQueuedCompletionStatus(
                self.port,
                &mut transferred,
                &mut key,
                &mut overlapped_ptr,
                0,
            )
        };
        let err = if res == 0 {
            let error = unsafe { GetLastError() };
            match error {
                WAIT_TIMEOUT => return,
                ERROR_HANDLE_EOF => None,
                _ => Some(IoError::from_raw_os_error(error as _)),
            }
        } else {
            None
        };
        if let Some(overlapped) = waker::OverlappedWaker::from_raw(overlapped_ptr as _) {
            if let Some(err) = err {
                overlapped.set_err(err);
            }
            if let Some(waker) = overlapped.take_waker() {
                waker.wake();
            }
        }
    }
}

impl Drop for IoPort {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.port) };
    }
}
