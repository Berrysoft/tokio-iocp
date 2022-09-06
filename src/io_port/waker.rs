use crate::*;
use once_cell::unsync::OnceCell;
use std::{cell::RefCell, rc::Rc, task::Waker};
use windows_sys::Win32::System::IO::OVERLAPPED;

#[repr(C)]
pub struct OverlappedWaker {
    overlapped: OVERLAPPED,
    waker: RefCell<Option<Waker>>,
    err: RefCell<Option<IoError>>,
}

impl OverlappedWaker {
    pub fn new() -> Self {
        Self {
            overlapped: unsafe { std::mem::zeroed() },
            waker: RefCell::new(None),
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
    ) -> Result<(&Rc<OverlappedWaker>, *mut OVERLAPPED), E> {
        let ptr = match self.ptr.get() {
            Some(ptr) => {
                ptr.set_waker(waker);
                (ptr, Rc::as_ptr(ptr) as *mut OVERLAPPED)
            }
            None => {
                let overlapped = self.ptr.get_or_init(|| Rc::new(OverlappedWaker::new()));
                overlapped.set_waker(waker);
                let ptr = overlapped.clone().leak() as *mut OVERLAPPED;
                f(ptr)?;
                (overlapped, ptr)
            }
        };
        Ok(ptr)
    }
}
