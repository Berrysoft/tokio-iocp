use crate::op::*;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6};
use windows_sys::Win32::Networking::WinSock::WSARecvFrom;

const ADDR_IN_SIZE: usize = std::mem::size_of::<SOCKADDR_IN>();
const ADDR_IN6_SIZE: usize = std::mem::size_of::<SOCKADDR_IN6>();
const MAX_ADDR_SIZE: usize = ADDR_IN6_SIZE;

pub struct RecvFrom<T: WithWsaBufMut> {
    buffer: T,
    addr_buffer: [u8; MAX_ADDR_SIZE],
    addr_size: i32,
}

impl<T: WithWsaBufMut> RecvFrom<T> {
    pub fn new(buffer: T::Buffer) -> Self {
        Self {
            buffer: T::new(buffer),
            addr_buffer: [0; MAX_ADDR_SIZE],
            addr_size: MAX_ADDR_SIZE as _,
        }
    }

    unsafe fn get_addr(&self) -> SocketAddr {
        match self.addr_size as usize {
            ADDR_IN_SIZE => {
                let addr = self.addr_buffer.as_ptr() as *const SOCKADDR_IN;
                let addr = addr.as_ref().unwrap();
                SocketAddr::V4(SocketAddrV4::new(
                    Ipv4Addr::from(addr.sin_addr.S_un.S_addr),
                    addr.sin_port,
                ))
            }
            ADDR_IN6_SIZE => {
                let addr = self.addr_buffer.as_ptr() as *const SOCKADDR_IN6;
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
}

impl<T: WithWsaBufMut> IocpOperation for RecvFrom<T> {
    type Output = (usize, SocketAddr);
    type Buffer = T::Buffer;

    unsafe fn operate(&mut self, handle: usize, overlapped_ptr: *mut OVERLAPPED) -> IoResult<()> {
        let res = self.buffer.with_wsa_buf_mut(|ptr, len| {
            let mut flags = 0;
            let mut received = 0;
            WSARecvFrom(
                handle,
                ptr,
                len as _,
                &mut received,
                &mut flags,
                self.addr_buffer.as_mut_ptr() as _,
                &mut self.addr_size,
                overlapped_ptr,
                None,
            )
        });
        wsa_result(res)
    }

    fn set_buf_len(&mut self, len: usize) {
        self.buffer.set_len(len)
    }

    fn result(&mut self, res: usize) -> BufResult<Self::Output, Self::Buffer> {
        let addr = unsafe { self.get_addr() };
        (Ok((res, addr)), self.buffer.take_buf())
    }

    fn error(&mut self, err: IoError) -> BufResult<Self::Output, Self::Buffer> {
        (Err(err), self.buffer.take_buf())
    }
}

pub type RecvFromOne<T> = RecvFrom<BufWrapper<T>>;
pub type RecvFromVectored<T> = RecvFrom<VectoredBufWrapper<T>>;
