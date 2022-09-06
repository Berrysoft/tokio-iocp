pub mod file;
pub mod socket;

use crate::*;
use std::{
    cell::{LazyCell, OnceCell},
    ptr::null_mut,
    task::Waker,
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, GetLastError, ERROR_HANDLE_EOF, INVALID_HANDLE_VALUE, WAIT_TIMEOUT},
    System::IO::{CreateIoCompletionPort, GetQueuedCompletionStatus, OVERLAPPED},
};

#[thread_local]
pub(crate) static IO_PORT: LazyCell<IoPort> = LazyCell::new(|| IoPort::new().unwrap());

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

    pub fn poll(&self) -> IoResult<()> {
        let mut transferred = 0;
        let mut key = 0;
        let mut overlapped_ptr = null_mut();
        let res = unsafe {
            GetQueuedCompletionStatus(
                self.port,
                &mut transferred,
                &mut key,
                &mut overlapped_ptr,
                0, // INFINITE
            )
        };
        if res == 0 {
            let error = unsafe { GetLastError() };
            match error {
                WAIT_TIMEOUT => return Ok(()),
                ERROR_HANDLE_EOF => {}
                _ => return Err(IoError::from_raw_os_error(error as _)),
            }
        }
        let mut overlapped = OverlappedWaker::from_raw(overlapped_ptr as _);
        if let Some(waker) = overlapped.take_waker() {
            waker.wake()
        }
        Ok(())
    }
}

impl Drop for IoPort {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.port) };
    }
}

#[repr(C)]
pub struct OverlappedWaker {
    pub overlapped: OVERLAPPED,
    pub waker: Option<Waker>,
}

impl OverlappedWaker {
    pub fn new() -> Self {
        Self {
            overlapped: unsafe { std::mem::zeroed() },
            waker: None,
        }
    }

    pub fn set_waker(&mut self, waker: Waker) -> Option<Waker> {
        self.waker.replace(waker)
    }

    pub fn take_waker(&mut self) -> Option<Waker> {
        self.waker.take()
    }

    pub fn leak(self) -> *mut Self {
        let this = Box::new(self);
        Box::into_raw(this)
    }

    pub fn from_raw(p: *mut Self) -> Box<Self> {
        unsafe { Box::from_raw(p) }
    }
}

pub struct OverlappedWakerWrapper {
    ptr: OnceCell<*mut OverlappedWaker>,
}

impl OverlappedWakerWrapper {
    pub fn new() -> Self {
        Self {
            ptr: OnceCell::new(),
        }
    }

    pub fn get_and_try_op<E>(
        &self,
        waker: Waker,
        f: impl FnOnce(*mut OVERLAPPED) -> Result<(), E>,
    ) -> Result<(bool, *mut OVERLAPPED), E> {
        let ptr = match self.ptr.get() {
            Some(&ptr) => {
                if let Some(overlapped) = unsafe { ptr.as_mut() } {
                    overlapped.set_waker(waker);
                }
                (false, ptr as *mut OVERLAPPED)
            }
            None => {
                let mut overlapped = Box::new(OverlappedWaker::new());
                overlapped.set_waker(waker);
                let overlapped_ptr = overlapped.leak();
                self.ptr.set(overlapped_ptr).unwrap();
                let ptr = overlapped_ptr as *mut OVERLAPPED;
                f(ptr)?;
                (true, ptr)
            }
        };
        Ok(ptr)
    }
}
