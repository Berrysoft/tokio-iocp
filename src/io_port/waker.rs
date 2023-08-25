use crate::*;
use std::{
    cell::{RefCell, RefMut},
    ops::Deref,
    task::Waker,
};
use windows_sys::Win32::System::IO::OVERLAPPED;

#[repr(C)]
pub struct OverlappedWakerBase {
    overlapped: OVERLAPPED,
    waker: RefCell<Option<Waker>>,
    err: RefCell<Option<IoError>>,
}

impl OverlappedWakerBase {
    pub fn new() -> Self {
        Self {
            overlapped: unsafe { std::mem::zeroed() },
            waker: RefCell::new(None),
            err: RefCell::new(None),
        }
    }

    pub fn set_waker(&self, waker: Waker) {
        self.waker.borrow_mut().replace(waker);
    }

    pub fn take_waker(&self) -> Option<Waker> {
        self.waker.borrow_mut().take()
    }

    pub fn set_err(&self, err: IoError) {
        self.err.borrow_mut().replace(err);
    }

    pub fn take_err(&self) -> Option<IoError> {
        self.err.borrow_mut().take()
    }
}

#[repr(C)]
pub struct OverlappedWaker<T> {
    base: OverlappedWakerBase,
    buffer: RefCell<Option<T>>,
}

impl<T> OverlappedWaker<T> {
    pub fn new(buffer: T) -> Self {
        Self {
            base: OverlappedWakerBase::new(),
            buffer: RefCell::new(Some(buffer)),
        }
    }

    pub fn buffer_mut(&self) -> RefMut<Option<T>> {
        self.buffer.borrow_mut()
    }

    pub fn take_buffer(&self) -> T {
        self.buffer.take().unwrap()
    }
}

impl<T> Deref for OverlappedWaker<T> {
    type Target = OverlappedWakerBase;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
