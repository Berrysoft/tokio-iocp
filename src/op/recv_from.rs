use std::net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6};

use windows_sys::Win32::Networking::WinSock::{WSARecvFrom, WSABUF};

use crate::{buf::*, op::*, *};

const ADDR_IN_SIZE: usize = std::mem::size_of::<SOCKADDR_IN>();
const ADDR_IN6_SIZE: usize = std::mem::size_of::<SOCKADDR_IN6>();
const MAX_ADDR_SIZE: usize = ADDR_IN6_SIZE;

unsafe fn exact_addr(buffer: &[u8; MAX_ADDR_SIZE], size: i32) -> SocketAddr {
    match size as usize {
        ADDR_IN_SIZE => {
            let addr = buffer.as_ptr() as *const SOCKADDR_IN;
            let addr = addr.as_ref().unwrap();
            SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::from(addr.sin_addr.S_un.S_addr),
                addr.sin_port,
            ))
        }
        ADDR_IN6_SIZE => {
            let addr = buffer.as_ptr() as *const SOCKADDR_IN6;
            let addr = addr.as_ref().unwrap();
            SocketAddr::V6(SocketAddrV6::new(
                Ipv6Addr::from(addr.sin6_addr.u.Byte),
                addr.sin6_port,
                addr.sin6_flowinfo,
                addr.Anonymous.sin6_scope_id,
            ))
        }
        _ => unimplemented!(),
    }
}

pub struct RecvFrom<T: IoBufMut> {
    buffer: T,
    addr_buffer: [u8; MAX_ADDR_SIZE],
    addr_size: i32,
}

impl<T: IoBufMut> RecvFrom<T> {
    pub fn new(buffer: T) -> Self {
        Self {
            buffer,
            addr_buffer: [0; MAX_ADDR_SIZE],
            addr_size: MAX_ADDR_SIZE as _,
        }
    }
}

impl<T: IoBufMut> IocpOperation for RecvFrom<T> {
    type Output = (usize, SocketAddr);
    type Buffer = T;

    unsafe fn operate(&mut self, handle: usize, overlapped_ptr: *mut OVERLAPPED) -> IoResult<()> {
        let buffer = WSABUF {
            len: self.buffer.buf_len() as _,
            buf: self.buffer.as_buf_mut_ptr() as _,
        };
        let mut flags = 0;
        let mut received = 0;
        let res = WSARecvFrom(
            handle,
            &buffer,
            1,
            &mut received,
            &mut flags,
            self.addr_buffer.as_mut_ptr() as _,
            &mut self.addr_size,
            overlapped_ptr,
            None,
        );
        wsa_result(res)
    }

    fn result(&mut self, res: usize) -> BufResult<Self::Output, Self::Buffer> {
        let addr = unsafe { exact_addr(&self.addr_buffer, self.addr_size) };
        (Ok((res, addr)), self.buffer.take())
    }

    fn error(&mut self, err: IoError) -> BufResult<Self::Output, Self::Buffer> {
        (Err(err), self.buffer.take())
    }
}

pub struct RecvFromVectored<T: IoBufMut> {
    buffer: Vec<T>,
    addr_buffer: [u8; MAX_ADDR_SIZE],
    addr_size: i32,
}

impl<T: IoBufMut> RecvFromVectored<T> {
    pub fn new(buffer: Vec<T>) -> Self {
        Self {
            buffer,
            addr_buffer: [0; MAX_ADDR_SIZE],
            addr_size: MAX_ADDR_SIZE as _,
        }
    }
}

impl<T: IoBufMut> IocpOperation for RecvFromVectored<T> {
    type Output = (usize, SocketAddr);
    type Buffer = Vec<T>;

    unsafe fn operate(&mut self, handle: usize, overlapped_ptr: *mut OVERLAPPED) -> IoResult<()> {
        let buffers = self
            .buffer
            .iter_mut()
            .map(|buf| WSABUF {
                len: buf.buf_len() as _,
                buf: buf.as_buf_mut_ptr() as _,
            })
            .collect::<Vec<_>>();
        let mut flags = 0;
        let mut received = 0;
        let res = WSARecvFrom(
            handle,
            buffers.as_ptr(),
            buffers.len() as _,
            &mut received,
            &mut flags,
            self.addr_buffer.as_mut_ptr() as _,
            &mut self.addr_size,
            overlapped_ptr,
            None,
        );
        wsa_result(res)
    }

    fn result(&mut self, res: usize) -> BufResult<Self::Output, Self::Buffer> {
        let addr = unsafe { exact_addr(&self.addr_buffer, self.addr_size) };
        (Ok((res, addr)), std::mem::take(&mut self.buffer))
    }

    fn error(&mut self, err: IoError) -> BufResult<Self::Output, Self::Buffer> {
        (Err(err), std::mem::take(&mut self.buffer))
    }
}
