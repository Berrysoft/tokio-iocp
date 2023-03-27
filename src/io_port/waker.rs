use crate::{op::IocpOperation, *};
use std::{
    cell::{RefCell, RefMut},
    task::Waker,
};
use windows_sys::Win32::System::IO::OVERLAPPED;

pub trait WakerOp {
    fn set_waker(&mut self, waker: Waker);

    fn take_waker(&mut self) -> Option<Waker>;

    fn set_err(&mut self, err: IoError);

    fn take_err(&mut self) -> Option<IoError>;

    unsafe fn op_ptr(&mut self) -> *mut ();
}

impl<T: WakerOp> WakerOp for &mut T {
    fn set_waker(&mut self, waker: Waker) {
        (**self).set_waker(waker)
    }

    fn take_waker(&mut self) -> Option<Waker> {
        (**self).take_waker()
    }

    fn set_err(&mut self, err: IoError) {
        (**self).set_err(err)
    }

    fn take_err(&mut self) -> Option<IoError> {
        (**self).take_err()
    }

    unsafe fn op_ptr(&mut self) -> *mut () {
        (**self).op_ptr()
    }
}

pub trait WakerOpExt: WakerOp {
    unsafe fn op_mut<Op>(&mut self) -> &mut Op {
        unsafe { &mut *(self.op_ptr() as *mut Op) }
    }
}

impl<T: WakerOp + ?Sized> WakerOpExt for T {}

#[repr(C)]
pub struct OverlappedWaker {
    overlapped: OVERLAPPED,
    waker: Box<RefCell<dyn WakerOp>>,
}

impl OverlappedWaker {
    pub fn new(waker: impl WakerOp + 'static) -> Self {
        Self {
            overlapped: unsafe { std::mem::zeroed() },
            waker: Box::new(RefCell::new(waker)),
        }
    }

    pub fn waker(&self) -> RefMut<dyn WakerOp> {
        self.waker.borrow_mut()
    }
}

pub struct IoWakerOp<Op: IocpOperation> {
    waker: Option<Waker>,
    err: Option<IoError>,
    op: Op,
}

impl<Op: IocpOperation> IoWakerOp<Op> {
    pub fn new(op: Op) -> Self {
        Self {
            waker: None,
            err: None,
            op,
        }
    }
}

impl<Op: IocpOperation> WakerOp for IoWakerOp<Op> {
    fn set_waker(&mut self, waker: Waker) {
        self.waker.replace(waker);
    }

    fn take_waker(&mut self) -> Option<Waker> {
        self.waker.take()
    }

    fn set_err(&mut self, err: IoError) {
        self.err.replace(err);
    }

    fn take_err(&mut self) -> Option<IoError> {
        self.err.take()
    }

    unsafe fn op_ptr(&mut self) -> *mut () {
        &self.op as *const Op as *mut Op as *mut ()
    }
}
