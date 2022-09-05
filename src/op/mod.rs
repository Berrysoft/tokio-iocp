pub mod fs;
pub mod net;

use crate::*;
use windows_sys::Win32::{
    Foundation::{GetLastError, ERROR_IO_PENDING},
    System::IO::OVERLAPPED,
};

pub trait IocpOperation: Unpin {
    type Buffer;

    unsafe fn operate(&mut self, handle: usize, overlapped_ptr: *mut OVERLAPPED) -> IoResult<()>;

    fn take_buffer(&mut self) -> Self::Buffer;
}
