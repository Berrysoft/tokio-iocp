pub mod read_at;
pub mod recv;
pub mod send;
pub mod write_at;

use crate::*;
use windows_sys::Win32::{
    Foundation::{GetLastError, ERROR_HANDLE_EOF, ERROR_IO_INCOMPLETE, ERROR_IO_PENDING},
    Networking::WinSock::{WSAGetLastError, WSA_IO_INCOMPLETE},
    System::IO::OVERLAPPED,
};

pub trait IocpOperation: Unpin {
    type Output: Unpin;
    type Buffer: Unpin;

    unsafe fn operate(&mut self, handle: usize, overlapped_ptr: *mut OVERLAPPED) -> IoResult<()>;

    fn result(&mut self, res: usize) -> BufResult<Self::Output, Self::Buffer>;
    fn error(&mut self, err: IoError) -> BufResult<Self::Output, Self::Buffer>;
}

pub(crate) unsafe fn win32_result(res: i32) -> IoResult<()> {
    if res == 0 {
        let error = GetLastError();
        match error {
            ERROR_IO_PENDING | ERROR_IO_INCOMPLETE | ERROR_HANDLE_EOF => Ok(()),
            _ => Err(IoError::from_raw_os_error(error as _)),
        }
    } else {
        Ok(())
    }
}

pub(crate) unsafe fn wsa_result(res: i32) -> IoResult<()> {
    if res == 0 {
        let error = WSAGetLastError();
        match error {
            WSA_IO_INCOMPLETE => Ok(()),
            _ => Err(IoError::from_raw_os_error(error as _)),
        }
    } else {
        Ok(())
    }
}
