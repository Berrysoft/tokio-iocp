pub mod read_at;
pub mod recv;
pub mod recv_from;
pub mod send;
pub mod send_to;
pub mod write_at;

use crate::{buf::*, *};
use std::net::SocketAddr;
use windows_sys::Win32::{
    Foundation::{GetLastError, ERROR_HANDLE_EOF, ERROR_IO_INCOMPLETE, ERROR_IO_PENDING},
    Networking::WinSock::{
        WSAGetLastError, AF_INET, AF_INET6, SOCKADDR, SOCKADDR_IN, SOCKADDR_IN6, WSABUF,
        WSA_IO_INCOMPLETE,
    },
    System::IO::OVERLAPPED,
};

pub trait IocpOperation: Unpin {
    type Output: Unpin;
    type Buffer: Unpin;

    unsafe fn operate(&mut self, handle: usize, overlapped_ptr: *mut OVERLAPPED) -> IoResult<()>;

    fn result(&mut self, res: usize) -> BufResult<Self::Output, Self::Buffer>;
    fn error(&mut self, err: IoError) -> BufResult<Self::Output, Self::Buffer>;
}

pub trait WithBuf: Unpin {
    type Buffer: Unpin;

    fn new(buffer: Self::Buffer) -> Self;
    fn with_buf<R>(&self, f: impl FnOnce(*const u8, usize) -> R) -> R;
    fn take_buf(&mut self) -> Self::Buffer;
}

pub trait WithBufMut: WithBuf {
    fn with_buf_mut<R>(&mut self, f: impl FnOnce(*mut u8, usize) -> R) -> R;
}

pub struct BufWrapper<T> {
    buffer: Option<T>,
}

impl<T: IoBuf> WithBuf for BufWrapper<T> {
    type Buffer = T;

    fn new(buffer: Self::Buffer) -> Self {
        Self {
            buffer: Some(buffer),
        }
    }

    fn with_buf<R>(&self, f: impl FnOnce(*const u8, usize) -> R) -> R {
        let buffer = self.buffer.as_ref().unwrap();
        f(buffer.as_buf_ptr(), buffer.buf_len())
    }

    fn take_buf(&mut self) -> Self::Buffer {
        self.buffer.take().unwrap()
    }
}

impl<T: IoBufMut> WithBufMut for BufWrapper<T> {
    fn with_buf_mut<R>(&mut self, f: impl FnOnce(*mut u8, usize) -> R) -> R {
        let buffer = self.buffer.as_mut().unwrap();
        f(buffer.as_buf_mut_ptr(), buffer.buf_len())
    }
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

pub trait WithWsaBuf: WithBuf {
    fn with_wsa_buf<R>(&self, f: impl FnOnce(*const WSABUF, usize) -> R) -> R;
}

pub trait WithWsaBufMut: WithBufMut + WithWsaBuf {
    fn with_wsa_buf_mut<R>(&mut self, f: impl FnOnce(*const WSABUF, usize) -> R) -> R;
}

impl<T: IoBuf> WithWsaBuf for BufWrapper<T> {
    fn with_wsa_buf<R>(&self, f: impl FnOnce(*const WSABUF, usize) -> R) -> R {
        let buffer = self.buffer.as_ref().unwrap();
        let buffer = WSABUF {
            len: buffer.buf_len() as _,
            buf: buffer.as_buf_ptr() as _,
        };
        f(&buffer, 1)
    }
}

impl<T: IoBufMut> WithWsaBufMut for BufWrapper<T> {
    fn with_wsa_buf_mut<R>(&mut self, f: impl FnOnce(*const WSABUF, usize) -> R) -> R {
        let buffer = self.buffer.as_mut().unwrap();
        let buffer = WSABUF {
            len: buffer.buf_len() as _,
            buf: buffer.as_buf_mut_ptr(),
        };
        f(&buffer, 1)
    }
}

pub struct VectoredBufWrapper<T> {
    buffer: Vec<T>,
}

impl<T: IoBuf> WithBuf for VectoredBufWrapper<T> {
    type Buffer = Vec<T>;

    fn new(buffer: Self::Buffer) -> Self {
        Self { buffer }
    }

    fn with_buf<R>(&self, _f: impl FnOnce(*const u8, usize) -> R) -> R {
        unimplemented!()
    }

    fn take_buf(&mut self) -> Self::Buffer {
        std::mem::take(&mut self.buffer)
    }
}

impl<T: IoBuf> WithWsaBuf for VectoredBufWrapper<T> {
    fn with_wsa_buf<R>(&self, f: impl FnOnce(*const WSABUF, usize) -> R) -> R {
        let buffers = self
            .buffer
            .iter()
            .map(|buf| WSABUF {
                len: buf.buf_len() as _,
                buf: buf.as_buf_ptr() as _,
            })
            .collect::<Vec<_>>();
        f(buffers.as_ptr(), buffers.len())
    }
}

impl<T: IoBufMut> WithBufMut for VectoredBufWrapper<T> {
    fn with_buf_mut<R>(&mut self, _f: impl FnOnce(*mut u8, usize) -> R) -> R {
        unimplemented!()
    }
}

impl<T: IoBufMut> WithWsaBufMut for VectoredBufWrapper<T> {
    fn with_wsa_buf_mut<R>(&mut self, f: impl FnOnce(*const WSABUF, usize) -> R) -> R {
        let buffers = self
            .buffer
            .iter_mut()
            .map(|buf| WSABUF {
                len: buf.buf_len() as _,
                buf: buf.as_buf_mut_ptr() as _,
            })
            .collect::<Vec<_>>();
        f(buffers.as_ptr(), buffers.len())
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
