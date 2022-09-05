pub mod fs;
pub mod net;

use crate::{buf::IoBuf, *};
use std::task::Poll;
use windows_sys::Win32::{
    Foundation::{GetLastError, ERROR_IO_PENDING},
    System::IO::OVERLAPPED,
};

pub trait IocpOperation: Unpin {
    type Buffer: IoBuf;

    unsafe fn operate(
        &self,
        handle: usize,
        buffer: &mut Self::Buffer,
        overlapped_ptr: *mut OVERLAPPED,
    ) -> Poll<IoResult<u32>>;
}
