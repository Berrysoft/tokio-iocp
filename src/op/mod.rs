pub mod fs;
pub mod net;

use crate::*;
use windows_sys::Win32::{
    Foundation::{GetLastError, ERROR_IO_PENDING},
    System::IO::OVERLAPPED,
};

pub trait IocpOperation: Unpin {
    type Output: Unpin;
    type Buffer: Unpin;

    unsafe fn operate(&mut self, handle: usize, overlapped_ptr: *mut OVERLAPPED) -> IoResult<()>;

    fn result(&mut self, res: usize) -> BufResult<Self::Output, Self::Buffer>;
    fn error(&mut self, err: IoError) -> BufResult<Self::Output, Self::Buffer>;
}
