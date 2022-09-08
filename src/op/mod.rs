pub mod accept;
pub mod connect;
pub mod read_at;
pub mod recv;
pub mod recv_from;
pub mod send;
pub mod send_to;
pub mod write_at;

use crate::{buf::*, *};
use std::ptr::null_mut;
use windows_sys::{
    core::GUID,
    Win32::{
        Foundation::{GetLastError, ERROR_HANDLE_EOF, ERROR_IO_INCOMPLETE, ERROR_IO_PENDING},
        Networking::WinSock::{WSAIoctl, SIO_GET_EXTENSION_FUNCTION_POINTER, SOCKADDR},
        System::IO::OVERLAPPED,
    },
};

pub trait IocpOperation: Unpin {
    type Output: Unpin;
    type Buffer: Unpin;

    unsafe fn operate(&mut self, handle: usize, overlapped_ptr: *mut OVERLAPPED) -> IoResult<()>;
    fn set_buf_len(&mut self, len: usize);

    fn result(&mut self, res: usize) -> BufResult<Self::Output, Self::Buffer>;
    fn error(&mut self, err: IoError) -> BufResult<Self::Output, Self::Buffer>;
}

pub unsafe fn win32_result(res: i32) -> IoResult<()> {
    if res == 0 {
        let error = GetLastError();
        match error {
            0 | ERROR_IO_PENDING | ERROR_IO_INCOMPLETE | ERROR_HANDLE_EOF => Ok(()),
            _ => Err(IoError::from_raw_os_error(error as _)),
        }
    } else {
        Ok(())
    }
}

pub const fn guid_from_u128(uuid: u128) -> GUID {
    GUID {
        data1: (uuid >> 96) as u32,
        data2: (uuid >> 80 & 0xffff) as u16,
        data3: (uuid >> 64 & 0xffff) as u16,
        data4: (uuid as u64).to_be_bytes(),
    }
}

pub unsafe fn get_wsa_fn<F>(handle: usize, fguid: GUID) -> IoResult<Option<F>> {
    let mut fptr = None;
    let mut returned = 0;
    let res = WSAIoctl(
        handle,
        SIO_GET_EXTENSION_FUNCTION_POINTER,
        &fguid as *const _ as _,
        std::mem::size_of_val(&fguid) as _,
        &mut fptr as *const _ as _,
        std::mem::size_of::<usize>() as _,
        &mut returned,
        null_mut(),
        None,
    );
    if res == 0 {
        Ok(fptr)
    } else {
        Err(IoError::last_os_error())
    }
}
