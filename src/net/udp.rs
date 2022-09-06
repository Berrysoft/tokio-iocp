use crate::{buf::*, net::socket::Socket, *};
use std::net::SocketAddr;
use windows_sys::Win32::Networking::WinSock::{IPPROTO_UDP, SOCK_DGRAM};

pub struct UdpSocket {
    inner: Socket,
}

impl UdpSocket {
    pub fn bind(addr: impl Into<SocketAddr>) -> IoResult<Self> {
        Ok(Self {
            inner: Socket::bind(addr.into(), SOCK_DGRAM, IPPROTO_UDP)?,
        })
    }

    pub async fn connect(addr: impl Into<SocketAddr>) -> IoResult<Self> {
        let addr = addr.into();
        let this = Self {
            inner: Socket::bind_any_like(addr, SOCK_DGRAM, IPPROTO_UDP)?,
        };
        this.inner.connect(addr).await?;
        Ok(this)
    }

    pub fn local_addr(&self) -> IoResult<SocketAddr> {
        self.inner.local_addr()
    }

    pub async fn recv<T: IoBufMut>(&self, buffer: T) -> BufResult<usize, T> {
        self.inner.recv(buffer).await
    }

    pub async fn recv_vectored<T: IoBufMut>(&self, buffer: Vec<T>) -> BufResult<usize, Vec<T>> {
        self.inner.recv_vectored(buffer).await
    }

    pub async fn send<T: IoBuf>(&self, buffer: T) -> BufResult<usize, T> {
        self.inner.send(buffer).await
    }

    pub async fn send_vectored<T: IoBuf>(&self, buffer: Vec<T>) -> BufResult<usize, Vec<T>> {
        self.inner.send_vectored(buffer).await
    }

    pub async fn recv_from<T: IoBufMut>(&self, buffer: T) -> BufResult<(usize, SocketAddr), T> {
        self.inner.recv_from(buffer).await
    }

    pub async fn recv_from_vectored<T: IoBufMut>(
        &self,
        buffer: Vec<T>,
    ) -> BufResult<(usize, SocketAddr), Vec<T>> {
        self.inner.recv_from_vectored(buffer).await
    }

    pub async fn send_to<T: IoBuf>(&self, buffer: T, addr: SocketAddr) -> BufResult<usize, T> {
        self.inner.send_to(buffer, addr).await
    }

    pub async fn send_to_vectored<T: IoBuf>(
        &self,
        buffer: Vec<T>,
        addr: SocketAddr,
    ) -> BufResult<usize, Vec<T>> {
        self.inner.send_to_vectored(buffer, addr).await
    }
}
