use crate::*;
use std::{cell::RefCell, task::Waker};
use windows_sys::Win32::System::IO::OVERLAPPED;

#[repr(C)]
pub struct OverlappedWaker {
    overlapped: OVERLAPPED,
    waker: RefCell<Option<Waker>>,
    err: RefCell<Option<IoError>>,
}

impl OverlappedWaker {
    pub fn new(waker: Waker) -> Self {
        Self {
            overlapped: unsafe { std::mem::zeroed() },
            waker: RefCell::new(Some(waker)),
            err: RefCell::new(None),
        }
    }

    pub fn set_waker(&self, waker: Waker) {
        self.waker.replace(Some(waker));
    }

    pub fn take_waker(&self) -> Option<Waker> {
        self.waker.take()
    }

    pub fn set_err(&self, err: IoError) {
        self.err.replace(Some(err));
    }

    pub fn take_err(&self) -> Option<IoError> {
        self.err.take()
    }

    pub fn as_ptr(&self) -> *const OVERLAPPED {
        &self.overlapped
    }
}
