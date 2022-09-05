use crate::{buf::*, net::socket::Socket, *};
use std::net::SocketAddr;
use windows_sys::Win32::Networking::WinSock::SOCK_DGRAM;

pub struct UdpSocket {
    inner: Socket,
}

impl UdpSocket {
    pub fn bind(addr: SocketAddr) -> IoResult<Self> {
        Ok(Self {
            inner: Socket::bind(addr, SOCK_DGRAM)?,
        })
    }

    pub fn connect(&self, addr: SocketAddr) -> IoResult<()> {
        self.inner.connect(addr)
    }

    pub async fn recv<T: IoBufMut>(&self, buffer: T) -> BufResult<usize, T> {
        self.inner.recv(buffer).await
    }

    pub async fn send<T: IoBuf>(&self, buffer: T) -> BufResult<usize, T> {
        self.inner.send(buffer).await
    }
}
