use crate::{op::IocpOperation, *};
use std::{cell::RefCell, task::Waker};
use windows_sys::Win32::System::IO::OVERLAPPED;

pub trait WakerOp {
    fn set_waker(&self, waker: Waker);

    fn take_waker(&self) -> Option<Waker>;

    fn set_err(&self, err: IoError);

    fn take_err(&self) -> Option<IoError>;

    fn op_ptr(&self) -> *mut ();
}

impl<T: WakerOp> WakerOp for &T {
    fn set_waker(&self, waker: Waker) {
        (*self).set_waker(waker)
    }

    fn take_waker(&self) -> Option<Waker> {
        (*self).take_waker()
    }

    fn set_err(&self, err: IoError) {
        (*self).set_err(err)
    }

    fn take_err(&self) -> Option<IoError> {
        (*self).take_err()
    }

    fn op_ptr(&self) -> *mut () {
        (*self).op_ptr()
    }
}

pub trait WakerOpExt: WakerOp {
    #[allow(clippy::mut_from_ref)]
    unsafe fn op_mut<Op>(&self) -> &mut Op {
        unsafe { &mut *(self.op_ptr() as *mut Op) }
    }
}

impl<T: WakerOp + ?Sized> WakerOpExt for T {}

#[repr(C)]
pub struct OverlappedWaker {
    overlapped: OVERLAPPED,
    waker: Box<dyn WakerOp>,
}

impl OverlappedWaker {
    pub fn new(waker: impl WakerOp + 'static) -> Self {
        Self {
            overlapped: unsafe { std::mem::zeroed() },
            waker: Box::new(waker),
        }
    }

    pub fn waker(&self) -> &dyn WakerOp {
        self.waker.as_ref()
    }
}

pub struct IoWakerOp<Op: IocpOperation> {
    waker: RefCell<Option<Waker>>,
    err: RefCell<Option<IoError>>,
    op: Op,
}

impl<Op: IocpOperation> IoWakerOp<Op> {
    pub fn new(op: Op) -> Self {
        Self {
            waker: RefCell::new(None),
            err: RefCell::new(None),
            op,
        }
    }
}

impl<Op: IocpOperation> WakerOp for IoWakerOp<Op> {
    fn set_waker(&self, waker: Waker) {
        self.waker.replace(Some(waker));
    }

    fn take_waker(&self) -> Option<Waker> {
        self.waker.take()
    }

    fn set_err(&self, err: IoError) {
        self.err.replace(Some(err));
    }

    fn take_err(&self) -> Option<IoError> {
        self.err.take()
    }

    fn op_ptr(&self) -> *mut () {
        &self.op as *const Op as *mut Op as *mut ()
    }
}
