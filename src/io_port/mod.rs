pub mod file;
pub mod socket;

use crate::*;
use std::{
    cell::{LazyCell, OnceCell},
    ptr::null_mut,
    rc::Rc,
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
        if let Some(overlapped) = OverlappedWaker::from_raw(overlapped_ptr as _) {
            let overlapped =
                unsafe { (Rc::as_ptr(&overlapped) as *mut OverlappedWaker).as_mut() }.unwrap();
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

#[repr(C)]
pub struct OverlappedWaker {
    overlapped: OVERLAPPED,
    waker: Option<Waker>,
    err: IoResult<()>,
}

impl OverlappedWaker {
    pub fn new() -> Self {
        Self {
            overlapped: unsafe { std::mem::zeroed() },
            waker: None,
            err: Ok(()),
        }
    }

    pub fn set_waker(&mut self, waker: Waker) -> Option<Waker> {
        self.waker.replace(waker)
    }

    pub fn take_waker(&mut self) -> Option<Waker> {
        self.waker.take()
    }

    pub fn set_err(&mut self, err: IoError) {
        self.err = Err(err);
    }

    pub fn take_err(&mut self) -> IoResult<()> {
        std::mem::replace(&mut self.err, Ok(()))
    }

    pub fn leak(self: Rc<Self>) -> *const Self {
        Rc::into_raw(self)
    }

    pub fn from_raw(p: *const Self) -> Option<Rc<Self>> {
        if p.is_null() {
            None
        } else {
            Some(unsafe { Rc::from_raw(p) })
        }
    }
}

pub struct OverlappedWakerWrapper {
    ptr: OnceCell<Rc<OverlappedWaker>>,
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
            Some(ptr) => {
                let ptr = Rc::as_ptr(ptr) as *mut OverlappedWaker;
                if let Some(overlapped) = unsafe { ptr.as_mut() } {
                    overlapped.set_waker(waker);
                }
                (false, ptr as *mut OVERLAPPED)
            }
            None => {
                let overlapped = self
                    .ptr
                    .get_or_init(|| Rc::new(OverlappedWaker::new()))
                    .clone();
                let overlapped_ptr = overlapped.leak() as *mut OverlappedWaker;
                unsafe { overlapped_ptr.as_mut() }.unwrap().set_waker(waker);
                let ptr = overlapped_ptr as *mut OVERLAPPED;
                f(ptr)?;
                (true, ptr)
            }
        };
        Ok(ptr)
    }
}
