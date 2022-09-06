mod buf_wrapper;
pub mod read_at;
pub mod recv;
pub mod recv_from;
pub mod send;
pub mod send_to;
pub mod write_at;

pub use buf_wrapper::*;

use crate::{buf::*, *};
use std::net::SocketAddr;
use windows_sys::Win32::{
    Foundation::{GetLastError, ERROR_HANDLE_EOF, ERROR_IO_INCOMPLETE, ERROR_IO_PENDING},
    Networking::WinSock::{
        WSAGetLastError, AF_INET, AF_INET6, SOCKADDR, SOCKADDR_IN, SOCKADDR_IN6, WSA_IO_INCOMPLETE,
    },
    System::IO::OVERLAPPED,
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
            ERROR_IO_PENDING | ERROR_IO_INCOMPLETE | ERROR_HANDLE_EOF => Ok(()),
            _ => Err(IoError::from_raw_os_error(error as _)),
        }
    } else {
        Ok(())
    }
}

pub unsafe fn wsa_result(res: i32) -> IoResult<()> {
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

pub unsafe fn wsa_exact_addr<T>(addr: SocketAddr, f: impl FnOnce(*const SOCKADDR, i32) -> T) -> T {
    match addr {
        SocketAddr::V4(addr) => {
            let native_addr = SOCKADDR_IN {
                sin_family: AF_INET as _,
                sin_port: addr.port(),
                sin_addr: std::mem::transmute(addr.ip().octets()),
                sin_zero: std::mem::zeroed(),
            };
            f(
                std::ptr::addr_of!(native_addr) as _,
                std::mem::size_of_val(&native_addr) as _,
            )
        }
        SocketAddr::V6(addr) => {
            let native_addr = SOCKADDR_IN6 {
                sin6_family: AF_INET6 as _,
                sin6_port: addr.port(),
                sin6_flowinfo: 0,
                sin6_addr: std::mem::transmute(addr.ip().octets()),
                Anonymous: std::mem::zeroed(),
            };
            f(
                std::ptr::addr_of!(native_addr) as _,
                std::mem::size_of_val(&native_addr) as _,
            )
        }
    }
}
